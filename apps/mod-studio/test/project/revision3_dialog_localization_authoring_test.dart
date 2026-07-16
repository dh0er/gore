import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_localization_authoring.dart';

import '../support/revision3_voice_content_fixture.dart';

const _locId = 'GRD_263_ASGHAN_OPEN_INFO_06_02';
const _otherLocalizationId = '77777777777777777777777777777777';
const _unknownEntityId = 'ABCDEFABCDEFABCDEFABCDEFABCDEF12';

AuthoringWorkingHead _head(String digit) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{'byte_len': 321, 'sha256': digit * 64},
      }),
    );

Revision3DialogLocalizationEditAuthoringService _service({
  required Future<AuthoringRevision3DialogLocalizationEditSeed> Function({
    required String expectedProjectId,
    required int expectedProjectRevision,
    required String localizationId,
    required int expectedLocalizationRevision,
    required String expectedLocId,
  })
  loadExactSeed,
  required Future<Revision3DialogLocalizationEditPublication> Function({
    required String expectedProjectId,
    required int expectedProjectRevision,
    required Revision3DialogLocalizationEditTechnicalPlan plan,
  })
  publishTechnicalPlan,
  List<int> revisions = const <int>[7],
  int candidateCount = 0,
  List<Revision3ContentIndex>? contentIndexes,
}) {
  var read = 0;
  return Revision3DialogLocalizationEditAuthoringService(
    loadContentIndex: () async {
      if (contentIndexes != null) {
        final index =
            contentIndexes[read < contentIndexes.length
                ? read
                : contentIndexes.length - 1];
        read++;
        return index;
      }
      final revision =
          revisions[read < revisions.length ? read : revisions.length - 1];
      read++;
      return _contentIndex(revision: revision, candidateCount: candidateCount);
    },
    loadExactSeed: loadExactSeed,
    publishTechnicalPlan: publishTechnicalPlan,
  );
}

Revision3ContentIndex _contentIndex({
  int revision = 7,
  int candidateCount = 0,
  bool duplicateFriendlyName = false,
  bool duplicateLine = false,
  String? localizationDisplayName = 'Asghan warning',
  String lineDisplayName = 'Mine entrance question',
  String speaker = 'Asghan',
  void Function(Map<String, Object?> json)? mutateJson,
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingSlotCandidateCount: candidateCount,
    duplicateLine: duplicateLine,
    lineDisplayName: lineDisplayName,
    speaker: speaker,
  );
  final entities = (json['entities']! as List<Object?>)
      .cast<Map<String, Object?>>();
  final localization = entities.singleWhere(
    (entity) => entity['id'] == revision3VoiceContentLocalizationId,
  );
  if (localizationDisplayName != null) {
    localization['display_name'] = localizationDisplayName;
  }
  localization['revision'] = 4;
  final summary = (localization['summary']! as Map).cast<String, Object?>();
  final data = (summary['data']! as Map).cast<String, Object?>();
  data['locales'] = <Object?>['de', 'en'];

  if (duplicateFriendlyName) {
    final duplicate = (jsonDecode(jsonEncode(localization)) as Map)
        .cast<String, Object?>();
    duplicate['id'] = _otherLocalizationId;
    duplicate['display_name'] = 'asghan warning';
    duplicate['revision'] = 1;
    final duplicateOrigin = (duplicate['origin']! as Map)
        .cast<String, Object?>();
    duplicateOrigin['authored_runtime_id'] = 'GORE_OTHER_WARNING';
    final duplicateSummary = (duplicate['summary']! as Map)
        .cast<String, Object?>();
    final duplicateData = (duplicateSummary['data']! as Map)
        .cast<String, Object?>();
    duplicateData['loc_id'] = 'GORE_OTHER_WARNING';
    entities.add(duplicate);
    entities.sort(
      (left, right) =>
          (left['id']! as String).compareTo(right['id']! as String),
    );
    final counts = (json['entity_counts']! as Map).cast<String, Object?>();
    counts['localization_entry'] = 2;
  }
  mutateJson?.call(json);
  return Revision3ContentIndex.fromJsonObject(json);
}

Revision3ContentIndex _contentIndexWithTwoLineFacts({
  required String firstName,
  required String firstSpeaker,
  required String secondName,
  required String secondSpeaker,
}) => _contentIndex(
  duplicateLine: true,
  localizationDisplayName: 'Guard warning',
  mutateJson: (json) {
    final entities = (json['entities']! as List<Object?>)
        .cast<Map<String, Object?>>();
    void setFacts(String id, String name, String speaker) {
      final line = entities.singleWhere((entity) => entity['id'] == id);
      line['display_name'] = name;
      final summary = (line['summary']! as Map).cast<String, Object?>();
      final data = (summary['data']! as Map).cast<String, Object?>();
      data['speaker_hint'] = speaker;
    }

    setFacts(revision3VoiceContentLineId, firstName, firstSpeaker);
    setFacts(revision3VoiceContentDuplicateLineId, secondName, secondSpeaker);
  },
);

AuthoringRevision3DialogLocalizationEditSeed _exactSeed({
  String projectId = revision3VoiceContentProjectId,
  int projectRevision = 7,
  String localizationId = revision3VoiceContentLocalizationId,
  int localizationRevision = 4,
  String locId = _locId,
  String de = 'Bleib stehen!',
  String en = 'Stop right there!',
  bool deVoiceSlot = true,
  int deCandidateCount = 0,
  String headDigit = 'b',
  String lineId = revision3VoiceContentLineId,
  int lineRevision = 2,
  String lineDisplayName = 'Asghan warning line',
  String? lineSpeaker = 'Asghan',
  Iterable<String>? backlinkVoiceSlotLocales,
}) {
  final expectedHead = _head(headDigit);
  final voiceSlotLocales =
      (backlinkVoiceSlotLocales ?? <String>[if (deVoiceSlot) 'de']).toList()
        ..sort();
  final request = AuthoringRevision3DialogLocalizationEditSeedRequestV1(
    expectedHead: expectedHead,
    localizationId: localizationId,
    expectedLocalizationRevision: localizationRevision,
    expectedLocId: locId,
  );
  return AuthoringRevision3DialogLocalizationEditSeed.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'read_only',
      'head_json': expectedHead.canonicalJson,
      'project_id': projectId,
      'project_revision': projectRevision,
      'localization_id': localizationId,
      'localization_revision': localizationRevision,
      'loc_id': locId,
      'locales': <Object?>[
        <String, Object?>{
          'locale': 'de',
          'text': de,
          'voice_slot_present': deVoiceSlot,
          'candidate_count': deCandidateCount,
        },
        <String, Object?>{
          'locale': 'en',
          'text': en,
          'voice_slot_present': false,
          'candidate_count': 0,
        },
      ],
      'line_backlinks': <Object?>[
        <String, Object?>{
          'line_id': lineId,
          'line_revision': lineRevision,
          'display_name': lineDisplayName,
          'speaker_hint': lineSpeaker,
          'voice_slot_locales': <Object?>[...voiceSlotLocales],
        },
      ],
      'content_authority': 'read_only_exact_current_localization_edit_seed',
      'build_status': 'not_evaluated',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_applicable',
    },
    request: request,
  );
}

void main() {
  test(
    'catalog exposes friendly opaque choices and disambiguates duplicates',
    () {
      final catalog = Revision3DialogLocalizationEditCatalog.fromContentIndex(
        _contentIndex(duplicateFriendlyName: true),
      );

      expect(catalog.projectId, revision3VoiceContentProjectId);
      expect(catalog.projectRevision, 7);
      expect(catalog.choices.map((choice) => choice.displayLabel), <String>[
        'Asghan warning (1)',
        'asghan warning (2)',
      ]);
      for (final choice in catalog.choices) {
        expect(choice.stableKey, matches(RegExp(r'^[0-9a-f]{24}$')));
        expect(
          choice.stableKey,
          isNot(contains(revision3VoiceContentLocalizationId)),
        );
        expect(choice.stableKey, isNot(contains(_locId)));
        expect(choice.matches('EN'), isTrue);
      }
    },
  );

  test(
    'standard Voice projection replaces its LocID label with the linked line',
    () {
      final catalog = Revision3DialogLocalizationEditCatalog.fromContentIndex(
        _contentIndex(localizationDisplayName: null),
      );
      final choice = catalog.choices.single;

      expect(choice.displayLabel, 'Mine entrance question');
      expect(choice.matches('mine entrance'), isTrue);
      expect(choice.matches('ASGHAN'), isTrue);
      expect(choice.matches(_locId), isFalse);
      expect(choice.displayLabel, isNot(contains(_locId)));
    },
  );

  test(
    'technical label without one visible linked line uses safe fallback',
    () {
      final catalog = Revision3DialogLocalizationEditCatalog.fromContentIndex(
        _contentIndex(
          localizationDisplayName: '',
          mutateJson: (json) {
            final entities = (json['entities']! as List<Object?>)
                .cast<Map<String, Object?>>();
            final line = entities.singleWhere(
              (entity) => entity['id'] == revision3VoiceContentLineId,
            );
            final references = (line['references']! as List<Object?>)
                .cast<Map<String, Object?>>();
            final localization = references.singleWhere(
              (reference) => reference['role'] == 'dialog_localization',
            );
            final target = (localization['target']! as Map)
                .cast<String, Object?>();
            target['project_id'] = 'ffffffffffffffffffffffffffffffff';
            localization['resolution'] = 'foreign_project';
          },
        ),
      );
      final choice = catalog.choices.single;

      expect(choice.displayLabel, 'Project text');
      expect(choice.matches(_locId), isFalse);
      expect(choice.matches(revision3VoiceContentLocalizationId), isFalse);
      expect(choice.matches('Mine entrance question'), isFalse);
    },
  );

  test('catalog search includes only visible associated line facts', () {
    final catalog = Revision3DialogLocalizationEditCatalog.fromContentIndex(
      _contentIndex(
        localizationDisplayName: 'Guard warning',
        lineDisplayName: 'Mine entrance question',
        speaker: 'Ore keeper',
      ),
    );
    final choice = catalog.choices.single;

    expect(choice.matches('entrance question'), isTrue);
    expect(choice.matches('ORE KEEPER'), isTrue);
    expect(choice.matches(revision3VoiceContentProjectId), isFalse);
    expect(choice.matches(revision3VoiceContentLocalizationId), isFalse);
    expect(choice.matches(revision3VoiceContentLineId), isFalse);
    expect(choice.matches(revision3VoiceContentSlotId), isFalse);
    expect(choice.matches(_locId), isFalse);
    expect(choice.matches(choice.stableKey), isFalse);
    expect(choice.matches('ASGHAN'), isFalse);
    expect(
      choice.visibleContextLabelFor('entrance question'),
      'Mine entrance question · Ore keeper',
    );
    expect(
      choice.visibleContextLabelFor('ORE KEEPER'),
      'Ore keeper · Mine entrance question',
    );
    final visibleContext = choice.visibleContextLabelFor('ORE KEEPER')!;
    for (final technicalValue in <String>[
      revision3VoiceContentProjectId,
      revision3VoiceContentLocalizationId,
      revision3VoiceContentLineId,
      revision3VoiceContentSlotId,
      _locId,
      choice.stableKey,
    ]) {
      expect(visibleContext, isNot(contains(technicalValue)));
    }
  });

  test('visible context follows the matching linked line fact', () {
    final catalog = Revision3DialogLocalizationEditCatalog.fromContentIndex(
      _contentIndexWithTwoLineFacts(
        firstName: 'Mine entrance question',
        firstSpeaker: 'Ore keeper',
        secondName: 'Tunnel greeting',
        secondSpeaker: 'Viper',
      ),
    );
    final choice = catalog.choices.single;

    expect(choice.matches('VIPER'), isTrue);
    expect(choice.visibleContextLabelFor('VIPER'), 'Viper · Tunnel greeting');
    expect(
      choice.visibleContextLabelFor('tunnel greeting'),
      'Tunnel greeting · Viper',
    );
  });

  test('technical tokens discard the whole line fact fail-closed', () {
    final catalog = Revision3DialogLocalizationEditCatalog.fromContentIndex(
      _contentIndexWithTwoLineFacts(
        firstName: 'Tunnel $_unknownEntityId greeting',
        firstSpeaker: 'Compromised ${_locId.toLowerCase()} voice',
        secondName: 'Safe camp warning',
        secondSpeaker: 'Viper',
      ),
    );
    final choice = catalog.choices.single;

    expect(choice.matches(_unknownEntityId), isFalse);
    expect(choice.matches(_locId), isFalse);
    expect(choice.matches('Tunnel'), isFalse);
    expect(choice.matches('Compromised'), isFalse);
    expect(choice.matches('Viper'), isTrue);
    expect(
      choice.visibleContextLabelFor(_unknownEntityId),
      'Safe camp warning · Viper',
    );
    expect(choice.visibleContextLabelFor(_locId), 'Safe camp warning · Viper');
    expect(choice.visibleContextLabelFor('Viper'), 'Viper · Safe camp warning');
  });

  test('foreign missing and wrong-kind localization backlinks are ignored', () {
    final mutations = <String, void Function(Map<String, Object?>)>{
      'foreign': (reference) {
        final target = (reference['target']! as Map).cast<String, Object?>();
        target['project_id'] = 'ffffffffffffffffffffffffffffffff';
        reference['resolution'] = 'foreign_project';
      },
      'missing': (reference) {
        final target = (reference['target']! as Map).cast<String, Object?>();
        target['entity_id'] = 'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';
        reference['resolution'] = 'missing_entity';
      },
      'wrong kind': (reference) {
        final target = (reference['target']! as Map).cast<String, Object?>();
        target['entity_id'] = revision3VoiceContentSlotId;
        reference['resolution'] = 'kind_mismatch';
      },
    };

    for (final mutation in mutations.entries) {
      final catalog = Revision3DialogLocalizationEditCatalog.fromContentIndex(
        _contentIndex(
          localizationDisplayName: 'Guard warning',
          lineDisplayName: 'Hidden broken line',
          speaker: 'Broken speaker',
          mutateJson: (json) {
            final entities = (json['entities']! as List<Object?>)
                .cast<Map<String, Object?>>();
            final line = entities.singleWhere(
              (entity) => entity['id'] == revision3VoiceContentLineId,
            );
            final references = (line['references']! as List<Object?>)
                .cast<Map<String, Object?>>();
            final reference = references.singleWhere(
              (item) => item['role'] == 'dialog_localization',
            );
            mutation.value(reference);
          },
        ),
      );

      expect(
        catalog.choices.single.matches('Hidden broken line'),
        isFalse,
        reason: mutation.key,
      );
      expect(
        catalog.choices.single.matches('Broken speaker'),
        isFalse,
        reason: mutation.key,
      );
    }
  });

  test(
    'seed preserves full text and exposes only author-facing Voice locks',
    () async {
      final longText = 'Ganz langer Dialog. ' * 80;
      final service = _service(
        loadExactSeed:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async => _exactSeed(de: longText, deCandidateCount: 1),
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async => throw UnimplementedError(),
        candidateCount: 1,
      );
      final catalog = await service.loadCatalog();
      final seed = await service.loadSeed(
        catalog: catalog,
        choice: catalog.choices.single,
      );

      expect(seed.locale('de')?.text, longText);
      expect(seed.locale('de')?.textLocked, isTrue);
      expect(seed.locale('de')?.canRemove, isFalse);
      expect(seed.locale('en')?.textLocked, isFalse);
      expect(seed.lineBacklinks.single.displayName, 'Asghan warning line');
      expect(seed.lineBacklinks.single.speakerLabel, 'Asghan');
    },
  );

  test(
    'publish reopens exact current state and emits only the bounded edit',
    () async {
      Revision3DialogLocalizationEditTechnicalPlan? receivedPlan;
      var exactReads = 0;
      final service = _service(
        loadExactSeed:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async {
              exactReads++;
              return _exactSeed();
            },
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              expect(expectedProjectId, revision3VoiceContentProjectId);
              expect(expectedProjectRevision, 7);
              receivedPlan = plan;
              return Revision3DialogLocalizationEditPublication(
                projectId: revision3VoiceContentProjectId,
                projectRevision: 8,
                localizationId: revision3VoiceContentLocalizationId,
                localizationRevision: 5,
                addedLocales: const <String>['fr'],
                removedLocales: const <String>['en'],
              );
            },
      );
      final catalog = await service.loadCatalog();
      final seed = await service.loadSeed(
        catalog: catalog,
        choice: catalog.choices.single,
      );

      final publication = await service.publish(
        seed: seed,
        input: Revision3DialogLocalizationEditInput(
          texts: const <String, String>{
            'de': 'Bleib bitte stehen!',
            'fr': 'Arrête-toi !',
          },
        ),
      );

      expect(exactReads, 2, reason: 'publish must reopen the exact seed');
      expect(
        receivedPlan?.expectedHead.canonicalJson,
        _head('b').canonicalJson,
      );
      expect(receivedPlan?.localizationId, revision3VoiceContentLocalizationId);
      expect(receivedPlan?.expectedLocalizationRevision, 4);
      expect(receivedPlan?.expectedLocId, _locId);
      expect(receivedPlan?.texts, <String, String>{
        'de': 'Bleib bitte stehen!',
        'fr': 'Arrête-toi !',
      });
      expect(publication.projectRevision, 8);
    },
  );

  test(
    'same-revision exact head text or locale metadata drift is stale',
    () async {
      final freshDrifts =
          <String, AuthoringRevision3DialogLocalizationEditSeed>{
            'head': _exactSeed(headDigit: 'c'),
            'text': _exactSeed(de: 'Extern geÃ¤ndert'),
            'Voice metadata': _exactSeed(deCandidateCount: 1),
          };
      var publishCalls = 0;

      for (final drift in freshDrifts.entries) {
        var exactReads = 0;
        final service = _service(
          loadExactSeed:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required localizationId,
                required expectedLocalizationRevision,
                required expectedLocId,
              }) async {
                exactReads++;
                return exactReads == 1 ? _exactSeed() : drift.value;
              },
          publishTechnicalPlan:
              ({
                required expectedProjectId,
                required expectedProjectRevision,
                required plan,
              }) async {
                publishCalls++;
                throw UnimplementedError();
              },
        );
        final catalog = await service.loadCatalog();
        final seed = await service.loadSeed(
          catalog: catalog,
          choice: catalog.choices.single,
        );

        await expectLater(
          service.publish(
            seed: seed,
            input: Revision3DialogLocalizationEditInput(
              texts: const <String, String>{
                'de': 'Meine Ã„nderung',
                'en': 'Stop right there!',
              },
            ),
          ),
          throwsA(
            isA<Revision3DialogLocalizationEditStaleCheckpointException>(),
          ),
          reason: drift.key,
        );
        expect(exactReads, 2, reason: drift.key);
      }
      expect(publishCalls, 0);
    },
  );

  test(
    'same-revision backlink identity drift is stale despite equal search facts',
    () async {
      final original = _contentIndex(
        localizationDisplayName: 'Guard warning',
        lineDisplayName: 'Shared warning line',
        speaker: 'Gate guard',
      );
      final changed = _contentIndex(
        localizationDisplayName: 'Guard warning',
        lineDisplayName: 'Shared warning line',
        speaker: 'Gate guard',
        mutateJson: (json) {
          final entities = (json['entities']! as List<Object?>)
              .cast<Map<String, Object?>>();
          final line = entities.singleWhere(
            (entity) => entity['id'] == revision3VoiceContentLineId,
          );
          line['id'] = revision3VoiceContentDuplicateLineId;
          entities.sort(
            (left, right) =>
                (left['id']! as String).compareTo(right['id']! as String),
          );
        },
      );
      var exactReads = 0;
      var publishCalls = 0;
      final service = _service(
        contentIndexes: <Revision3ContentIndex>[original, changed],
        loadExactSeed:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async {
              exactReads++;
              return _exactSeed(
                lineId: exactReads == 1
                    ? revision3VoiceContentLineId
                    : revision3VoiceContentDuplicateLineId,
                lineDisplayName: 'Shared warning line',
                lineSpeaker: 'Gate guard',
              );
            },
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishCalls++;
              throw UnimplementedError();
            },
      );
      final catalog = await service.loadCatalog();
      final seed = await service.loadSeed(
        catalog: catalog,
        choice: catalog.choices.single,
      );

      await expectLater(
        service.publish(
          seed: seed,
          input: Revision3DialogLocalizationEditInput(
            texts: const <String, String>{
              'de': 'Meine Ã„nderung',
              'en': 'Stop right there!',
            },
          ),
        ),
        throwsA(isA<Revision3DialogLocalizationEditStaleCheckpointException>()),
      );
      expect(exactReads, 2, reason: 'catalog search facts must remain equal');
      expect(publishCalls, 0);
    },
  );

  test(
    'search fact fingerprint is deterministic across dialog line identities',
    () async {
      var publishCalls = 0;
      final service = _service(
        contentIndexes: <Revision3ContentIndex>[
          _contentIndexWithTwoLineFacts(
            firstName: 'Mine entrance',
            firstSpeaker: 'Ore keeper',
            secondName: 'Camp warning',
            secondSpeaker: 'Gate guard',
          ),
          _contentIndexWithTwoLineFacts(
            firstName: 'Camp warning',
            firstSpeaker: 'Gate guard',
            secondName: 'Mine entrance',
            secondSpeaker: 'Ore keeper',
          ),
        ],
        loadExactSeed:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async => _exactSeed(),
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishCalls++;
              return Revision3DialogLocalizationEditPublication(
                projectId: revision3VoiceContentProjectId,
                projectRevision: 8,
                localizationId: revision3VoiceContentLocalizationId,
                localizationRevision: 5,
                addedLocales: const <String>[],
                removedLocales: const <String>[],
              );
            },
      );
      final catalog = await service.loadCatalog();
      final seed = await service.loadSeed(
        catalog: catalog,
        choice: catalog.choices.single,
      );

      await service.publish(
        seed: seed,
        input: Revision3DialogLocalizationEditInput(
          texts: const <String, String>{
            'de': 'Neue Warnung',
            'en': 'Stop right there!',
          },
        ),
      );

      expect(publishCalls, 1);
    },
  );

  test('changed line or speaker search facts stale the checkpoint', () async {
    final original = _contentIndex(
      localizationDisplayName: 'Guard warning',
      lineDisplayName: 'Mine entrance',
      speaker: 'Ore keeper',
    );
    final changedIndexes = <Revision3ContentIndex>[
      _contentIndex(
        localizationDisplayName: 'Guard warning',
        lineDisplayName: 'Camp warning',
        speaker: 'Ore keeper',
      ),
      _contentIndex(
        localizationDisplayName: 'Guard warning',
        lineDisplayName: 'Mine entrance',
        speaker: 'Gate guard',
      ),
    ];
    var publishCalls = 0;

    for (final changed in changedIndexes) {
      final service = _service(
        contentIndexes: <Revision3ContentIndex>[original, changed],
        loadExactSeed:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async => _exactSeed(),
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishCalls++;
              throw UnimplementedError();
            },
      );
      final catalog = await service.loadCatalog();
      final seed = await service.loadSeed(
        catalog: catalog,
        choice: catalog.choices.single,
      );

      await expectLater(
        service.publish(
          seed: seed,
          input: Revision3DialogLocalizationEditInput(
            texts: const <String, String>{
              'de': 'Neue Warnung',
              'en': 'Stop right there!',
            },
          ),
        ),
        throwsA(isA<Revision3DialogLocalizationEditStaleCheckpointException>()),
      );
    }
    expect(publishCalls, 0);
  });

  test(
    'stale catalog and Voice-backed changes fail before publication',
    () async {
      var publishCalls = 0;
      final staleService = _service(
        revisions: const <int>[7, 8],
        loadExactSeed:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async => _exactSeed(),
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishCalls++;
              throw UnimplementedError();
            },
      );
      final staleCatalog = await staleService.loadCatalog();
      final staleSeed = await staleService.loadSeed(
        catalog: staleCatalog,
        choice: staleCatalog.choices.single,
      );
      await expectLater(
        staleService.publish(
          seed: staleSeed,
          input: Revision3DialogLocalizationEditInput(
            texts: const <String, String>{
              'de': 'Neu',
              'en': 'Stop right there!',
            },
          ),
        ),
        throwsA(isA<Revision3DialogLocalizationEditStaleCheckpointException>()),
      );

      final lockedService = _service(
        candidateCount: 1,
        loadExactSeed:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async => _exactSeed(deCandidateCount: 1),
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publishCalls++;
              throw UnimplementedError();
            },
      );
      final lockedCatalog = await lockedService.loadCatalog();
      final lockedSeed = await lockedService.loadSeed(
        catalog: lockedCatalog,
        choice: lockedCatalog.choices.single,
      );
      await expectLater(
        lockedService.publish(
          seed: lockedSeed,
          input: Revision3DialogLocalizationEditInput(
            texts: const <String, String>{
              'de': 'Verändert',
              'en': 'Stop right there!',
            },
          ),
        ),
        throwsA(isA<Revision3DialogLocalizationEditLockedVoiceTextException>()),
      );
      expect(publishCalls, 0);
    },
  );

  test('new or previously written text cannot silently become blank', () async {
    var publishCalls = 0;
    final service = _service(
      loadExactSeed:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async => _exactSeed(deVoiceSlot: false),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publishCalls++;
            throw UnimplementedError();
          },
    );
    final catalog = await service.loadCatalog();
    final seed = await service.loadSeed(
      catalog: catalog,
      choice: catalog.choices.single,
    );

    await expectLater(
      service.publish(
        seed: seed,
        input: Revision3DialogLocalizationEditInput(
          texts: const <String, String>{'de': ' ', 'en': 'Still present'},
        ),
      ),
      throwsFormatException,
    );
    await expectLater(
      service.publish(
        seed: seed,
        input: Revision3DialogLocalizationEditInput(
          texts: const <String, String>{
            'de': 'Bleib stehen!',
            'en': 'Stop right there!',
            'fr': ' ',
          },
        ),
      ),
      throwsFormatException,
    );
    expect(publishCalls, 0);
  });
}
