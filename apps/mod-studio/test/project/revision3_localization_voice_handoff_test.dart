import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_localization_voice_handoff.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  test('resolves exact Localization, DialogLine, VoiceSlot, and VoiceTake', () {
    final json = revision3VoiceContentIndexJsonFixture(
      existingSlotCandidateCount: 1,
    );
    _makeLocalizationAuthorable(json);
    final index = Revision3ContentIndex.fromJsonObject(json);
    final localization = index.entityById(revision3VoiceContentLocalizationId)!;
    final line = index.entityById(revision3VoiceContentLineId)!;
    final slot = index.entityById(revision3VoiceContentSlotId)!;
    final take = index.entities.singleWhere(
      (entity) => entity.kind == Revision3ContentEntityKind.voiceTake,
    );

    final localizationTarget = resolveRevision3LocalizationVoiceEntityHandoff(
      index: index,
      entity: localization,
    )!;
    expect(
      localizationTarget.localizationEntityId,
      revision3VoiceContentLocalizationId,
    );
    expect(localizationTarget.dialogLineEntityId, isNull);
    expect(localizationTarget.locale, isNull);

    final lineTarget = resolveRevision3LocalizationVoiceEntityHandoff(
      index: index,
      entity: line,
    )!;
    expect(lineTarget.localizationEntityId, localization.id);
    expect(lineTarget.dialogLineEntityId, line.id);
    expect(lineTarget.locale, isNull);

    final slotTarget = resolveRevision3LocalizationVoiceEntityHandoff(
      index: index,
      entity: slot,
    )!;
    expect(slotTarget.localizationEntityId, localization.id);
    expect(slotTarget.dialogLineEntityId, line.id);
    expect(slotTarget.locale, 'de');
    expect(slotTarget.voiceSlotEntityId, slot.id);
    expect(slotTarget.voiceTakeEntityId, isNull);

    final takeTarget = resolveRevision3LocalizationVoiceEntityHandoff(
      index: index,
      entity: take,
    )!;
    expect(takeTarget.localizationEntityId, localization.id);
    expect(takeTarget.dialogLineEntityId, line.id);
    expect(takeTarget.locale, 'de');
    expect(takeTarget.voiceSlotEntityId, slot.id);
    expect(takeTarget.voiceTakeEntityId, take.id);
  });

  test('refuses an ambiguous VoiceTake owner instead of choosing one', () {
    final json = revision3VoiceContentIndexJsonFixture(
      existingSlotCandidateCount: 1,
      duplicateLine: true,
    );
    _makeLocalizationAuthorable(json);
    final entities = (json['entities']! as List).cast<Map<String, Object?>>();
    final duplicateLine = entities.singleWhere(
      (entity) => entity['id'] == revision3VoiceContentDuplicateLineId,
    );
    final slot = entities.singleWhere(
      (entity) => entity['id'] == revision3VoiceContentSlotId,
    );
    final take = entities.singleWhere(
      (entity) => entity['kind'] == 'voice_take',
    );
    const secondSlotId = '77777777777777777777777777777777';
    final duplicateSummary = duplicateLine['summary']! as Map<String, Object?>;
    final duplicateData = duplicateSummary['data']! as Map<String, Object?>;
    duplicateData['voice_slot_locales'] = <Object?>['de'];
    (duplicateLine['references']! as List).add(
      _reference(
        role: 'dialog_voice_slot',
        qualifier: 'de',
        entityId: secondSlotId,
        expectedKind: 'voice_slot',
      ),
    );
    entities.add(<String, Object?>{
      ...slot,
      'id': secondSlotId,
      'display_name': 'Second German Voice',
      'origin': <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'second-voice-slot',
      },
      'references': <Object?>[
        _reference(
          role: 'voice_candidate',
          entityId: take['id']! as String,
          expectedKind: 'voice_take',
        ),
      ],
    });
    entities.sort(
      (left, right) =>
          (left['id']! as String).compareTo(right['id']! as String),
    );
    (json['entity_counts']! as Map<String, Object?>)['voice_slot'] = 2;
    final index = Revision3ContentIndex.fromJsonObject(json);
    final exactTake = index.entities.singleWhere(
      (entity) => entity.kind == Revision3ContentEntityKind.voiceTake,
    );

    expect(
      resolveRevision3LocalizationVoiceEntityHandoff(
        index: index,
        entity: exactTake,
      ),
      isNull,
    );
  });

  test('refuses an entity object from another project snapshot', () {
    final currentJson = revision3VoiceContentIndexJsonFixture();
    final olderJson = revision3VoiceContentIndexJsonFixture(revision: 6);
    _makeLocalizationAuthorable(currentJson);
    _makeLocalizationAuthorable(olderJson);
    final current = Revision3ContentIndex.fromJsonObject(currentJson);
    final older = Revision3ContentIndex.fromJsonObject(olderJson);
    final currentLine = current.entityById(revision3VoiceContentLineId)!;
    final olderLine = older.entityById(revision3VoiceContentLineId)!;

    expect(
      resolveRevision3LocalizationVoiceEntityHandoff(
        index: current,
        entity: currentLine,
      ),
      isNotNull,
    );
    expect(
      resolveRevision3LocalizationVoiceEntityHandoff(
        index: current,
        entity: olderLine,
      ),
      isNull,
    );
    expect(currentLine.revision, olderLine.revision);
  });

  test(
    'refuses non-authorable LocalizationEntries and every inherited graph',
    () {
      for (final mutation in <void Function(Map<String, Object?> localization)>[
        (localization) {
          localization['origin'] = <String, Object?>{
            'type': 'imported',
            'importer': 'tests',
            'source_seal': <String, Object?>{
              'byte_len': 10,
              'sha256': 'd' * 64,
            },
            'external_identity': 'imported-asghan',
          };
        },
        (localization) {
          final summary = (localization['summary']! as Map)
              .cast<String, Object?>();
          final data = (summary['data']! as Map).cast<String, Object?>();
          data['locales'] = <Object?>[];
        },
      ]) {
        final json = revision3VoiceContentIndexJsonFixture(
          existingSlotCandidateCount: 1,
        );
        _makeLocalizationAuthorable(json);
        final entities = (json['entities']! as List<Object?>)
            .cast<Map<String, Object?>>();
        mutation(
          entities.singleWhere(
            (entity) => entity['id'] == revision3VoiceContentLocalizationId,
          ),
        );
        final index = Revision3ContentIndex.fromJsonObject(json);

        for (final entity in index.entities.where(
          (entity) =>
              entity.kind == Revision3ContentEntityKind.localizationEntry ||
              entity.kind == Revision3ContentEntityKind.dialogLine ||
              entity.kind == Revision3ContentEntityKind.voiceSlot ||
              entity.kind == Revision3ContentEntityKind.voiceTake,
        )) {
          expect(
            resolveRevision3LocalizationVoiceEntityHandoff(
              index: index,
              entity: entity,
            ),
            isNull,
            reason: '${entity.kind.name} must inherit the rejection',
          );
        }
      }
    },
  );

  test('canonical Localization locale shape is enforced before handoff', () {
    final json = revision3VoiceContentIndexJsonFixture();
    final entities = (json['entities']! as List<Object?>)
        .cast<Map<String, Object?>>();
    final localization = entities.singleWhere(
      (entity) => entity['id'] == revision3VoiceContentLocalizationId,
    );
    final summary = (localization['summary']! as Map).cast<String, Object?>();
    final data = (summary['data']! as Map).cast<String, Object?>();
    data['locales'] = <Object?>['DE'];

    expect(
      () => Revision3ContentIndex.fromJsonObject(json),
      throwsA(isA<FormatException>()),
    );
  });

  test(
    'refuses a VoiceSlot whose structured locale disagrees with its edge',
    () {
      final json = revision3VoiceContentIndexJsonFixture();
      _makeLocalizationAuthorable(json);
      final entities = (json['entities']! as List<Object?>)
          .cast<Map<String, Object?>>();
      final slot = entities.singleWhere(
        (entity) => entity['id'] == revision3VoiceContentSlotId,
      );
      final summary = (slot['summary']! as Map).cast<String, Object?>();
      final data = (summary['data']! as Map).cast<String, Object?>();
      data['locale'] = 'en';
      final index = Revision3ContentIndex.fromJsonObject(json);
      final exactSlot = index.entityById(revision3VoiceContentSlotId)!;

      expect(exactSlot.summary.primaryIdentity, 'en');
      expect(exactSlot.problemCount, 0);
      expect(
        resolveRevision3LocalizationVoiceEntityHandoff(
          index: index,
          entity: exactSlot,
        ),
        isNull,
      );
    },
  );

  test('refuses a VoiceSlot with no dialog_voice_slot owner', () {
    final json = revision3VoiceContentIndexJsonFixture();
    _makeLocalizationAuthorable(json);
    final entities = (json['entities']! as List<Object?>)
        .cast<Map<String, Object?>>();
    final line = entities.singleWhere(
      (entity) => entity['id'] == revision3VoiceContentLineId,
    );
    final lineSummary = (line['summary']! as Map).cast<String, Object?>();
    final lineData = (lineSummary['data']! as Map).cast<String, Object?>();
    lineData['voice_slot_locales'] = <Object?>[];
    (line['references']! as List<Object?>).removeWhere(
      (reference) =>
          (reference as Map<String, Object?>)['role'] == 'dialog_voice_slot',
    );
    final index = Revision3ContentIndex.fromJsonObject(json);
    final exactSlot = index.entityById(revision3VoiceContentSlotId)!;

    expect(
      resolveRevision3LocalizationVoiceEntityHandoff(
        index: index,
        entity: exactSlot,
      ),
      isNull,
    );
  });

  test('refuses a VoiceSlot with multiple dialog_voice_slot owners', () {
    final json = revision3VoiceContentIndexJsonFixture(duplicateLine: true);
    _makeLocalizationAuthorable(json);
    final entities = (json['entities']! as List<Object?>)
        .cast<Map<String, Object?>>();
    final duplicateLine = entities.singleWhere(
      (entity) => entity['id'] == revision3VoiceContentDuplicateLineId,
    );
    final duplicateSummary = (duplicateLine['summary']! as Map)
        .cast<String, Object?>();
    final duplicateData = (duplicateSummary['data']! as Map)
        .cast<String, Object?>();
    duplicateData['voice_slot_locales'] = <Object?>['de'];
    (duplicateLine['references']! as List<Object?>).add(
      _reference(
        role: 'dialog_voice_slot',
        qualifier: 'de',
        entityId: revision3VoiceContentSlotId,
        expectedKind: 'voice_slot',
      ),
    );
    final index = Revision3ContentIndex.fromJsonObject(json);
    final exactSlot = index.entityById(revision3VoiceContentSlotId)!;

    expect(
      resolveRevision3LocalizationVoiceEntityHandoff(
        index: index,
        entity: exactSlot,
      ),
      isNull,
    );
  });

  test('refuses a VoiceSlot whose owner qualifier names another locale', () {
    final json = revision3VoiceContentIndexJsonFixture();
    _makeLocalizationAuthorable(json);
    final entities = (json['entities']! as List<Object?>)
        .cast<Map<String, Object?>>();
    final line = entities.singleWhere(
      (entity) => entity['id'] == revision3VoiceContentLineId,
    );
    final lineSummary = (line['summary']! as Map).cast<String, Object?>();
    final lineData = (lineSummary['data']! as Map).cast<String, Object?>();
    lineData['voice_slot_locales'] = <Object?>['en'];
    _dialogVoiceSlotReference(line)['qualifier'] = 'en';
    final index = Revision3ContentIndex.fromJsonObject(json);
    final exactSlot = index.entityById(revision3VoiceContentSlotId)!;

    expect(exactSlot.summary.primaryIdentity, 'de');
    expect(
      resolveRevision3LocalizationVoiceEntityHandoff(
        index: index,
        entity: exactSlot,
      ),
      isNull,
    );
  });

  test('refuses cross-project and unresolved VoiceSlot owner edges', () {
    for (final variant
        in <String, void Function(Map<String, Object?> reference)>{
          'cross-project': (reference) {
            final target = (reference['target']! as Map)
                .cast<String, Object?>();
            target['project_id'] = '99999999999999999999999999999999';
            reference['resolution'] = 'foreign_project';
          },
          'unresolved': (reference) {
            final target = (reference['target']! as Map)
                .cast<String, Object?>();
            target['entity_id'] = '88888888888888888888888888888888';
            reference['resolution'] = 'missing_entity';
          },
        }.entries) {
      final json = revision3VoiceContentIndexJsonFixture();
      _makeLocalizationAuthorable(json);
      final entities = (json['entities']! as List<Object?>)
          .cast<Map<String, Object?>>();
      final line = entities.singleWhere(
        (entity) => entity['id'] == revision3VoiceContentLineId,
      );
      variant.value(_dialogVoiceSlotReference(line));
      final index = Revision3ContentIndex.fromJsonObject(json);
      final exactSlot = index.entityById(revision3VoiceContentSlotId)!;

      expect(
        resolveRevision3LocalizationVoiceEntityHandoff(
          index: index,
          entity: exactSlot,
        ),
        isNull,
        reason: '${variant.key} edges are not exact owners',
      );
    }
  });
}

void _makeLocalizationAuthorable(Map<String, Object?> json) {
  final entities = (json['entities']! as List<Object?>)
      .cast<Map<String, Object?>>();
  final localization = entities.singleWhere(
    (entity) => entity['id'] == revision3VoiceContentLocalizationId,
  );
  final summary = (localization['summary']! as Map).cast<String, Object?>();
  final data = (summary['data']! as Map).cast<String, Object?>();
  data['locales'] = <Object?>['de'];
}

Map<String, Object?> _dialogVoiceSlotReference(
  Map<String, Object?> dialogLine,
) => (dialogLine['references']! as List<Object?>)
    .cast<Map<String, Object?>>()
    .singleWhere((reference) => reference['role'] == 'dialog_voice_slot');

Map<String, Object?> _reference({
  required String role,
  String? qualifier,
  required String entityId,
  required String expectedKind,
}) => <String, Object?>{
  'role': role,
  'qualifier': qualifier,
  'target': <String, Object?>{
    'project_id': revision3VoiceContentProjectId,
    'entity_id': entityId,
    'expected_kind': expectedKind,
  },
  'resolution': 'resolved',
};
