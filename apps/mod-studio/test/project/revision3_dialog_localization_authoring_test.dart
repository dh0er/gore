import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_localization_authoring.dart';

import '../support/revision3_voice_content_fixture.dart';

const _locId = 'GRD_263_ASGHAN_OPEN_INFO_06_02';
const _otherLocalizationId = '77777777777777777777777777777777';

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
}) {
  var read = 0;
  return Revision3DialogLocalizationEditAuthoringService(
    loadContentIndex: () async {
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
}) {
  final json = revision3VoiceContentIndexJsonFixture(
    revision: revision,
    existingSlotCandidateCount: candidateCount,
  );
  final entities = (json['entities']! as List<Object?>)
      .cast<Map<String, Object?>>();
  final localization = entities.singleWhere(
    (entity) => entity['id'] == revision3VoiceContentLocalizationId,
  );
  localization['display_name'] = 'Asghan warning';
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
  return Revision3ContentIndex.fromJsonObject(json);
}

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
}) {
  final expectedHead = _head('b');
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
          'line_id': revision3VoiceContentLineId,
          'line_revision': 2,
          'display_name': 'Asghan warning line',
          'speaker_hint': 'Asghan',
          'voice_slot_locales': <Object?>[if (deVoiceSlot) 'de'],
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
