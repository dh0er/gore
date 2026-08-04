import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_localization_authoring.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_production_queue.dart';

import '../support/revision3_voice_content_fixture.dart';

void main() {
  test('projects only absent project languages and intact existing slots', () {
    final catalogs = _catalogs(
      authoringLocales: const ['de', 'en'],
      localizationLocales: const ['de'],
      candidateStatuses: const [],
    );

    final queue = Revision3VoiceProductionQueue.fromCatalogs(
      localizationCatalog: catalogs.localization,
      voiceCatalog: catalogs.voice,
    );

    expect(queue.projectId, revision3VoiceContentProjectId);
    expect(queue.projectRevision, 7);
    expect(queue.voiceCatalogAvailable, isTrue);
    expect(queue.totalItemCount, 2);
    expect(queue.missingLanguageCount, 1);
    expect(queue.voiceSlotCount, 1);
    expect(queue.unboundVoiceSlotCount, 0);
    expect(queue.isPartial, isFalse);

    final language = queue.items
        .whereType<Revision3VoiceMissingLanguageQueueItem>()
        .single;
    expect(language.locale, 'en');
    expect(language.displayLabel, 'Mine entrance question');
    expect(language.choiceStableKey, isNotEmpty);
    expect(language.nextStep, Revision3VoiceProductionNextStep.addLanguage);
    expect(language.isActionable, isTrue);

    final voice = queue.items.whereType<Revision3VoiceSlotQueueItem>().single;
    expect(voice.lineId, revision3VoiceContentLineId);
    expect(voice.locale, 'de');
    expect(voice.localizationStableKey, language.choiceStableKey);
    expect(voice.hasLocalizationContext, isTrue);
    expect(voice.candidateCount, 0);
    expect(voice.nextStep, Revision3VoiceProductionNextStep.addRecording);
  });

  test('an absent VoiceSlot never becomes a missing-recording item', () {
    final catalogs = _catalogs(
      existingSlot: false,
      authoringLocales: const ['de'],
      localizationLocales: const ['de'],
    );

    final queue = Revision3VoiceProductionQueue.fromCatalogs(
      localizationCatalog: catalogs.localization,
      voiceCatalog: catalogs.voice,
    );

    expect(queue.totalItemCount, 0);
    expect(queue.voiceSlotCount, 0);
    expect(queue.items, isEmpty);
    expect(queue.countFor(Revision3VoiceProductionNextStep.addRecording), 0);
  });

  test('missing-language means language not added, never blank text', () {
    final catalogs = _catalogs(
      existingSlot: false,
      authoringLocales: const ['de', 'en', 'fr'],
      localizationLocales: const ['de'],
    );

    final queue = Revision3VoiceProductionQueue.fromCatalogs(
      localizationCatalog: catalogs.localization,
      voiceCatalog: catalogs.voice,
    );
    final languages = queue.items
        .whereType<Revision3VoiceMissingLanguageQueueItem>()
        .toList();

    expect(languages.map((item) => item.locale), ['en', 'fr']);
    expect(languages.map((item) => item.nextStep).toSet(), {
      Revision3VoiceProductionNextStep.addLanguage,
    });
  });

  test('zero takes asks for the first recording', () {
    final item = _singleVoiceItem(
      candidateStatuses: const [],
      targetResolution: 'resolved',
    );

    expect(item.candidateCount, 0);
    expect(item.approvedCount, 0);
    expect(item.selectionState, Revision3VoiceProductionSelectionState.none);
    expect(item.nextStep, Revision3VoiceProductionNextStep.addRecording);
  });

  test('no Approved take takes precedence over an invalid selection', () {
    final item = _singleVoiceItem(
      candidateStatuses: const ['recorded'],
      selected: true,
      targetResolution: 'resolved',
    );

    expect(
      item.selectionState,
      Revision3VoiceProductionSelectionState.selectedNotApproved,
    );
    expect(item.approvedCount, 0);
    expect(item.nextStep, Revision3VoiceProductionNextStep.reviewAndApprove);
  });

  test('an Approved take without selection asks to select it', () {
    final item = _singleVoiceItem(candidateStatuses: const ['approved']);

    expect(item.approvedCount, 1);
    expect(item.selectionState, Revision3VoiceProductionSelectionState.none);
    expect(item.nextStep, Revision3VoiceProductionNextStep.selectOrRepair);
  });

  test('a selected non-Approved take asks to repair the selection', () {
    final item = _singleVoiceItem(
      candidateStatuses: const ['recorded', 'approved'],
      selected: true,
    );

    expect(item.approvedCount, 1);
    expect(
      item.selectionState,
      Revision3VoiceProductionSelectionState.selectedNotApproved,
    );
    expect(item.nextStep, Revision3VoiceProductionNextStep.selectOrRepair);
  });

  for (final target in const ['unresolved', 'ambiguous']) {
    test('$target target asks for target resolution after selection', () {
      final item = _singleVoiceItem(
        candidateStatuses: const ['approved'],
        selected: true,
        targetResolution: target,
      );

      expect(
        item.selectionState,
        Revision3VoiceProductionSelectionState.selectedApproved,
      );
      expect(item.nextStep, Revision3VoiceProductionNextStep.resolveTarget);
    });
  }

  test('complete decisions stay complete despite unreviewed alternatives', () {
    final item = _singleVoiceItem(
      candidateStatuses: const ['approved', 'draft', 'recorded', 'reviewed'],
      selected: true,
      targetResolution: 'resolved',
    );

    expect(item.approvedCount, 1);
    expect(item.unreviewedAlternativeCount, 2);
    expect(item.hasUnreviewedAlternatives, isTrue);
    expect(item.productionDecisionsComplete, isTrue);
    expect(
      item.nextStep,
      Revision3VoiceProductionNextStep.productionDecisionsComplete,
    );
    expect(item.isActionable, isFalse);
  });

  test('existing slot remains visible without editable text context', () {
    final catalogs = _catalogs(
      localizationLocales: const [],
      candidateStatuses: const [],
    );

    final queue = Revision3VoiceProductionQueue.fromCatalogs(
      localizationCatalog: catalogs.localization,
      voiceCatalog: catalogs.voice,
    );
    final item = queue.items.whereType<Revision3VoiceSlotQueueItem>().single;

    expect(queue.missingLanguageCount, 0);
    expect(queue.voiceSlotCount, 1);
    expect(queue.unboundVoiceSlotCount, 1);
    expect(item.localizationStableKey, isNull);
    expect(item.hasLocalizationContext, isFalse);
    expect(item.nextStep, Revision3VoiceProductionNextStep.addRecording);
  });

  test('language work survives an unavailable Voice catalog', () {
    final catalogs = _catalogs(
      authoringLocales: const ['de', 'en'],
      localizationLocales: const ['de'],
    );

    final queue = Revision3VoiceProductionQueue.fromCatalogs(
      localizationCatalog: catalogs.localization,
      voiceCatalog: null,
    );

    expect(queue.voiceCatalogAvailable, isFalse);
    expect(queue.voiceSlotCount, 0);
    expect(queue.totalItemCount, 1);
    expect(queue.items.single, isA<Revision3VoiceMissingLanguageQueueItem>());
    expect(queue.isPartial, isFalse);
  });

  test('mismatched public checkpoints fail closed', () {
    final localization = _catalogs(revision: 7).localization;
    final voice = _catalogs(revision: 8).voice;

    expect(
      () => Revision3VoiceProductionQueue.fromCatalogs(
        localizationCatalog: localization,
        voiceCatalog: voice,
      ),
      throwsA(isA<Revision3VoiceProductionQueueCheckpointMismatch>()),
    );
  });

  test(
    'bounded retention is explicit and actionable work precedes complete',
    () {
      final catalogs = _catalogs(
        authoringLocales: const ['de', 'en'],
        localizationLocales: const ['de'],
        candidateStatuses: const ['approved', 'recorded'],
        selected: true,
        targetResolution: 'resolved',
      );

      final queue = Revision3VoiceProductionQueue.fromCatalogs(
        localizationCatalog: catalogs.localization,
        voiceCatalog: catalogs.voice,
        maxItems: 1,
      );

      expect(queue.totalItemCount, 2);
      expect(queue.items, hasLength(1));
      expect(queue.items.single, isA<Revision3VoiceMissingLanguageQueueItem>());
      expect(queue.omittedItemCount, 1);
      expect(queue.isPartial, isTrue);
      expect(queue.actionableCount, 1);
      expect(queue.productionDecisionsCompleteCount, 1);
      expect(
        queue.countFor(
          Revision3VoiceProductionNextStep.productionDecisionsComplete,
        ),
        1,
      );
    },
  );

  test(
    'zero retention still reports exact known totals and immutable rows',
    () {
      final catalogs = _catalogs(
        authoringLocales: const ['de', 'en'],
        localizationLocales: const ['de'],
      );
      final empty = Revision3VoiceProductionQueue.fromCatalogs(
        localizationCatalog: catalogs.localization,
        voiceCatalog: catalogs.voice,
        maxItems: 0,
      );
      final retained = Revision3VoiceProductionQueue.fromCatalogs(
        localizationCatalog: catalogs.localization,
        voiceCatalog: catalogs.voice,
      );

      expect(empty.items, isEmpty);
      expect(empty.totalItemCount, 2);
      expect(empty.omittedItemCount, 2);
      expect(empty.isPartial, isTrue);
      expect(
        () => retained.items.add(retained.items.first),
        throwsUnsupportedError,
      );
    },
  );

  test('retention limit itself is bounded', () {
    final catalogs = _catalogs();

    for (final invalid in [
      -1,
      Revision3VoiceProductionQueue.maximumMaxItems + 1,
    ]) {
      expect(
        () => Revision3VoiceProductionQueue.fromCatalogs(
          localizationCatalog: catalogs.localization,
          voiceCatalog: catalogs.voice,
          maxItems: invalid,
        ),
        throwsRangeError,
      );
    }
  });
}

Revision3VoiceSlotQueueItem _singleVoiceItem({
  required List<String> candidateStatuses,
  bool selected = false,
  String targetResolution = 'unresolved',
}) {
  final catalogs = _catalogs(
    authoringLocales: const ['de'],
    localizationLocales: const ['de'],
    candidateStatuses: candidateStatuses,
    selected: selected,
    targetResolution: targetResolution,
  );
  final queue = Revision3VoiceProductionQueue.fromCatalogs(
    localizationCatalog: catalogs.localization,
    voiceCatalog: catalogs.voice,
  );
  return queue.items.whereType<Revision3VoiceSlotQueueItem>().single;
}

({
  Revision3DialogLocalizationEditCatalog localization,
  Revision3VoiceCatalog voice,
})
_catalogs({
  int revision = 7,
  bool existingSlot = true,
  List<String> authoringLocales = const ['de', 'en'],
  List<String> localizationLocales = const ['de'],
  List<String> candidateStatuses = const [],
  bool selected = false,
  String targetResolution = 'unresolved',
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingDeSlot: existingSlot,
    existingSlotCandidateCount: candidateStatuses.length,
    existingSlotHasSelectedTake: selected,
    existingSlotTargetResolution: targetResolution,
  );
  json['authoring_locales'] = <Object?>[...authoringLocales]..sort();
  final localization = _entity(json, revision3VoiceContentLocalizationId);
  final localizationData = _summaryData(localization);
  localizationData['locales'] = <Object?>[...localizationLocales]..sort();

  final takes =
      (json['entities']! as List<Object?>)
          .whereType<Map<String, Object?>>()
          .where((entity) => entity['kind'] == 'voice_take')
          .toList()
        ..sort(
          (left, right) =>
              (left['id']! as String).compareTo(right['id']! as String),
        );
  expect(takes, hasLength(candidateStatuses.length));
  for (var index = 0; index < takes.length; index++) {
    _summaryData(takes[index])['status'] = candidateStatuses[index];
  }

  final index = Revision3ContentIndex.fromJsonObject(json);
  return (
    localization: Revision3DialogLocalizationEditCatalog.fromContentIndex(
      index,
    ),
    voice: Revision3VoiceCatalog.fromContentIndex(index),
  );
}

Map<String, Object?> _entity(Map<String, Object?> json, String id) =>
    (json['entities']! as List<Object?>)
        .whereType<Map<String, Object?>>()
        .singleWhere((entity) => entity['id'] == id);

Map<String, Object?> _summaryData(Map<String, Object?> entity) {
  final summary = (entity['summary']! as Map).cast<String, Object?>();
  return (summary['data']! as Map).cast<String, Object?>();
}
