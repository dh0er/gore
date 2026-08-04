import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_selection_authoring.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  test('catalog retains exact ordered friendly candidates and selection', () {
    final catalog = Revision3VoiceCatalog.fromContentIndex(
      revision3VoiceContentIndexFixture(
        existingSlotCandidateCount: 3,
        existingSlotHasSelectedTake: true,
      ),
    );
    final line = catalog.line(revision3VoiceContentLineId)!;
    final summary = line.slotSummaryForLocale('de')!;

    expect(summary.slotRevision, 1);
    expect(summary.candidateCount, 3);
    expect(summary.candidates.map((take) => take.id), <String>[
      '55000000000000000000000000000000',
      '55000000000000000000000000000001',
      '55000000000000000000000000000002',
    ]);
    expect(summary.candidates.map((take) => take.revision), [0, 0, 0]);
    expect(summary.candidates.map((take) => take.displayLabel), <String>[
      'Asghan take · 1 of 3',
      'Asghan take · 2 of 3',
      'Asghan take · 3 of 3',
    ]);
    expect(summary.candidates.first.isApproved, isTrue);
    expect(
      summary.candidates.skip(1).every((take) => !take.isApproved),
      isTrue,
    );
    expect(summary.selectedTakeId, summary.candidates.first.id);
  });

  test('catalog safely retains one take shared by distinct intact slots', () {
    final index = Revision3ContentIndex.fromJsonObject(_sharedTakeIndex());
    final catalog = Revision3VoiceCatalog.fromContentIndex(index);
    final slots = catalog.lines
        .map((line) => line.slotSummaryForLocale('de'))
        .whereType<Revision3VoiceExistingSlotSummary>()
        .toList(growable: false);

    expect(slots, hasLength(2));
    expect(slots.map((slot) => slot.candidates.single.id).toSet(), {
      '55000000000000000000000000000000',
    });
  });

  test(
    'technical plan permits clear and only a different Approved candidate',
    () {
      final catalog = Revision3VoiceCatalog.fromContentIndex(
        Revision3ContentIndex.fromJsonObject(_twoApprovedIndex()),
      );
      final summary = catalog
          .line(revision3VoiceContentLineId)!
          .slotSummaryForLocale('de')!;
      final clear = Revision3VoiceTakeSelectionTechnicalPlan.forCheckpoint(
        catalog: catalog,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        selectedTakeId: null,
      );
      expect(clear.expectedSlotRevision, 1);
      expect(clear.expectedSelectedTakeId, summary.candidates.first.id);
      expect(clear.selectedTakeId, isNull);

      final second = summary.candidates[1];
      final change = Revision3VoiceTakeSelectionTechnicalPlan.forCheckpoint(
        catalog: catalog,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        selectedTakeId: second.id,
      );
      expect(change.selectedTakeId, second.id);
      expect(
        () => Revision3VoiceTakeSelectionTechnicalPlan.forCheckpoint(
          catalog: catalog,
          lineId: revision3VoiceContentLineId,
          locale: 'de',
          selectedTakeId: summary.selectedTakeId,
        ),
        throwsFormatException,
      );

      final recorded = Revision3VoiceCatalog.fromContentIndex(
        revision3VoiceContentIndexFixture(
          existingSlotCandidateCount: 2,
          existingSlotHasSelectedTake: true,
        ),
      );
      final recordedSummary = recorded
          .line(revision3VoiceContentLineId)!
          .slotSummaryForLocale('de')!;
      expect(
        () => Revision3VoiceTakeSelectionTechnicalPlan.forCheckpoint(
          catalog: recorded,
          lineId: revision3VoiceContentLineId,
          locale: 'de',
          selectedTakeId: recordedSummary.candidates[1].id,
        ),
        throwsFormatException,
      );
    },
  );

  test('service refreshes exact index before publishing selection', () async {
    final index = Revision3ContentIndex.fromJsonObject(_twoApprovedIndex());
    final catalog = Revision3VoiceCatalog.fromContentIndex(index);
    final summary = catalog
        .line(revision3VoiceContentLineId)!
        .slotSummaryForLocale('de')!;
    Revision3VoiceTakeSelectionTechnicalPlan? received;
    final service = Revision3VoiceTakeSelectionAuthoringService(
      loadContentIndex: () async => index,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            received = plan;
            return Revision3VoiceTakeSelectionPublication(
              head: _publicationHead(),
              projectId: expectedProjectId,
              projectRevision: expectedProjectRevision + 1,
              lineId: plan.lineId,
              slotId: plan.slotId,
              slotRevision: plan.expectedSlotRevision + 1,
              locale: plan.locale,
              locId: plan.locId,
              previousSelectedTakeId: plan.expectedSelectedTakeId,
              selectedTakeId: plan.selectedTakeId,
            );
          },
    );

    final publication = await service.publish(
      checkpoint: catalog,
      lineId: revision3VoiceContentLineId,
      locale: 'de',
      selectedTakeId: summary.candidates[1].id,
    );

    expect(received, isNotNull);
    expect(publication.projectRevision, index.projectRevision + 1);
    expect(publication.selectedTakeId, summary.candidates[1].id);
  });

  test('service rejects stale catalog and mismatched publication', () async {
    final index = Revision3ContentIndex.fromJsonObject(_twoApprovedIndex());
    final catalog = Revision3VoiceCatalog.fromContentIndex(index);
    final selected = catalog
        .line(revision3VoiceContentLineId)!
        .slotSummaryForLocale('de')!
        .candidates[1]
        .id;
    final stale = Revision3VoiceTakeSelectionAuthoringService(
      loadContentIndex: () async => revision3VoiceContentIndexFixture(
        revision: index.projectRevision + 1,
        existingSlotCandidateCount: 2,
        existingSlotHasSelectedTake: true,
      ),
      publishTechnicalPlan: _unexpectedPublish,
    );
    await expectLater(
      stale.publish(
        checkpoint: catalog,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        selectedTakeId: selected,
      ),
      throwsA(isA<Revision3VoiceTakeSelectionStaleCheckpointException>()),
    );

    final mismatch = Revision3VoiceTakeSelectionAuthoringService(
      loadContentIndex: () async => index,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => Revision3VoiceTakeSelectionPublication(
            head: _publicationHead(),
            projectId: expectedProjectId,
            projectRevision: expectedProjectRevision + 2,
            lineId: plan.lineId,
            slotId: plan.slotId,
            slotRevision: plan.expectedSlotRevision + 1,
            locale: plan.locale,
            locId: plan.locId,
            previousSelectedTakeId: plan.expectedSelectedTakeId,
            selectedTakeId: plan.selectedTakeId,
          ),
    );
    await expectLater(
      mismatch.publish(
        checkpoint: catalog,
        lineId: revision3VoiceContentLineId,
        locale: 'de',
        selectedTakeId: selected,
      ),
      throwsA(isA<Revision3VoiceTakeSelectionRequiresReopenException>()),
    );
  });
}

Future<Revision3VoiceTakeSelectionPublication> _unexpectedPublish({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required Revision3VoiceTakeSelectionTechnicalPlan plan,
}) => throw StateError('publisher must not be called');

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

Map<String, Object?> _twoApprovedIndex() {
  final json = revision3VoiceContentIndexJsonFixture(
    existingSlotCandidateCount: 2,
    existingSlotHasSelectedTake: true,
  );
  final entities = (json['entities']! as List).cast<Map<String, Object?>>();
  for (final entity in entities) {
    if (entity['kind'] != 'voice_take') continue;
    final summary = (entity['summary']! as Map).cast<String, Object?>();
    final data = (summary['data']! as Map).cast<String, Object?>();
    data['status'] = 'approved';
    summary['data'] = data;
    entity['summary'] = summary;
  }
  return json;
}

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
  final secondSlotId = '77777777777777777777777777777777';
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
  return jsonDecode(jsonEncode(json)) as Map<String, Object?>;
}
