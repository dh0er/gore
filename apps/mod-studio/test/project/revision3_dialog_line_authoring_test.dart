import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dialog_line_authoring.dart';

const _projectId = '11111111111111111111111111111111';
const _unownedLocalizationId = '22222222222222222222222222222222';
const _ownedLocalizationId = '33333333333333333333333333333333';
const _ownerLineId = '44444444444444444444444444444444';
const _targetSha =
    'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

Map<String, Object?> _localizationEntity({
  required String id,
  required String displayName,
  required String locId,
  required List<String> locales,
  int revision = 4,
}) => <String, Object?>{
  'id': id,
  'kind': 'localization_entry',
  'display_name': displayName,
  'revision': revision,
  'origin': <String, Object?>{'type': 'new', 'authored_runtime_id': locId},
  'summary': <String, Object?>{
    'kind': 'localization_entry',
    'data': <String, Object?>{'loc_id': locId, 'locales': locales},
  },
  'references': <Object?>[],
  'asset_references': <Object?>[],
};

Map<String, Object?> _ownerLineEntity() => <String, Object?>{
  'id': _ownerLineId,
  'kind': 'dialog_line',
  'display_name': 'Existing owner line',
  'revision': 0,
  'origin': <String, Object?>{
    'type': 'new',
    'authored_runtime_id': 'GORE_DIALOG_EXISTING_OWNER',
  },
  'summary': <String, Object?>{
    'kind': 'dialog_line',
    'data': <String, Object?>{
      'speaker_hint': null,
      'voice_slot_locales': <Object?>[],
    },
  },
  'references': <Object?>[
    <String, Object?>{
      'role': 'dialog_localization',
      'qualifier': null,
      'target': <String, Object?>{
        'project_id': _projectId,
        'entity_id': _ownedLocalizationId,
        'expected_kind': 'localization_entry',
      },
      'resolution': 'resolved',
    },
  ],
  'asset_references': <Object?>[],
};

Revision3ContentIndex _contentIndex({
  int revision = 7,
  List<String> unownedLocales = const <String>['de', 'en'],
  List<Map<String, Object?>> extraEntities = const <Map<String, Object?>>[],
}) {
  final entities =
      <Map<String, Object?>>[
        _localizationEntity(
          id: _unownedLocalizationId,
          displayName: 'Reusable greeting',
          locId: 'GORE_REUSABLE_GREETING',
          locales: unownedLocales,
        ),
        _localizationEntity(
          id: _ownedLocalizationId,
          displayName: 'Already used greeting',
          locId: 'GORE_ALREADY_USED',
          locales: const <String>['de'],
        ),
        _ownerLineEntity(),
        ...extraEntities,
      ]..sort(
        (left, right) =>
            (left['id']! as String).compareTo(right['id']! as String),
      );
  final counts = <String, int>{};
  for (final entity in entities) {
    final kind = entity['kind']! as String;
    counts[kind] = (counts[kind] ?? 0) + 1;
  }
  final orderedCountKeys = <String>[
    for (final kind in Revision3ContentEntityKind.values)
      if (counts.containsKey(kind.wireName)) kind.wireName,
  ];

  return Revision3ContentIndex.fromJsonObject(<String, Object?>{
    'schema_revision': 1,
    'project_id': _projectId,
    'project_revision': revision,
    'project_name': 'Dialog authoring fixture',
    'project_version': '1.0.0',
    'project_author': 'tests',
    'target': <String, Object?>{
      'executable': <String, Object?>{
        'byte_len': 171698176,
        'sha256': _targetSha,
      },
    },
    'authoring_locales': <Object?>['en'],
    'entity_counts': <String, Object?>{
      for (final kind in orderedCountKeys) kind: counts[kind],
    },
    'entities': entities,
    'assets': <Object?>[],
  });
}

Revision3DialogLineEntryInput _createInput({
  String name = 'Asghan warning',
  String? speaker = 'Asghan',
  bool createVoiceSlot = true,
}) => Revision3DialogLineEntryInput.create(
  lineDisplayName: name,
  speakerHint: speaker,
  locale: 'de',
  text: 'Bleib stehen!',
  createVoiceSlot: createVoiceSlot,
);

String _planSeed(Revision3DialogLineEntryInput input, int revision) =>
    jsonEncode(<String, Object?>{
      'project_id': _projectId,
      'project_revision': revision,
      'mode': input.mode.name,
      'name': input.lineDisplayName,
      'speaker': input.speakerHint,
      'locale': input.locale,
      if (input case Revision3DialogLineEntryCreateInput(:final text))
        'text': text,
      if (input case Revision3DialogLineEntryReuseInput(:final localizationId))
        'localization_id': localizationId,
    });

String _generatedId(String domain, String seed, int counter) => sha256
    .convert(
      utf8.encode(
        'gore-mod-studio.r3-dialog-entry-$domain-v1\u0000$seed\u0000$counter',
      ),
    )
    .toString()
    .substring(0, 32);

AuthoringWorkingHead _dialogReadHead() =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{'byte_len': 321, 'sha256': 'a' * 64},
      }),
    );

Future<AuthoringRevision3DialogLocalizationReadResult>
_successfulLocalizationRead({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required String localizationId,
  required int expectedLocalizationRevision,
  required String expectedLocId,
}) async => _localizationReadResult(
  expectedProjectId: expectedProjectId,
  expectedProjectRevision: expectedProjectRevision,
  localizationId: localizationId,
  expectedLocalizationRevision: expectedLocalizationRevision,
  expectedLocId: expectedLocId,
);

AuthoringRevision3DialogLocalizationReadResult _localizationReadResult({
  required String expectedProjectId,
  required int expectedProjectRevision,
  required String localizationId,
  required int expectedLocalizationRevision,
  required String expectedLocId,
  String? actualProjectId,
  int? actualProjectRevision,
  List<Map<String, Object?>>? locales,
}) {
  final head = _dialogReadHead();
  final request = AuthoringRevision3DialogLocalizationReadRequestV1(
    expectedHead: head,
    localizationId: localizationId,
    expectedLocalizationRevision: expectedLocalizationRevision,
    expectedLocId: expectedLocId,
  );
  return AuthoringRevision3DialogLocalizationReadResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'read_only',
      'head_json': head.canonicalJson,
      'project_id': actualProjectId ?? expectedProjectId,
      'project_revision': actualProjectRevision ?? expectedProjectRevision,
      'localization_id': localizationId,
      'localization_revision': expectedLocalizationRevision,
      'loc_id': expectedLocId,
      'locales':
          locales ??
          <Object?>[
            <String, Object?>{
              'locale': 'de',
              'preview': 'Bleib stehen!',
              'truncated': false,
              'has_nonempty_text': true,
            },
            <String, Object?>{
              'locale': 'en',
              'preview': 'Stop there!',
              'truncated': false,
              'has_nonempty_text': true,
            },
          ],
      'content_authority': 'read_only_exact_current_localization',
      'build_status': 'not_evaluated',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_applicable',
    },
    request: request,
  );
}

void main() {
  test('catalog exposes only canonical unowned managed localizations', () {
    final catalog = Revision3DialogLineEntryCatalog.fromContentIndex(
      _contentIndex(),
    );

    expect(catalog.reusableLocalizations, hasLength(1));
    final choice = catalog.reusableLocalizations.single;
    expect(choice.id, _unownedLocalizationId);
    expect(choice.locId, 'GORE_REUSABLE_GREETING');
    expect(choice.locales, <String>['de', 'en']);
    expect(choice.matches('greeting'), isTrue);
    expect(catalog.localization(_ownedLocalizationId), isNull);
    expect(catalog.suggestedLocales, <String>['de', 'en']);
  });

  test('localization summary rejects noncanonical projected locales', () {
    expect(
      () => _contentIndex(unownedLocales: const <String>['DE']),
      throwsFormatException,
    );
  });

  test('technical planning is deterministic and retries occupied IDs', () {
    final input = _createInput();
    final seed = _planSeed(input, 7);
    final occupiedFirstLineId = _generatedId('line', seed, 0);
    final collision = _localizationEntity(
      id: occupiedFirstLineId,
      displayName: 'Collision fixture',
      locId: 'GORE_COLLISION_FIXTURE',
      locales: const <String>['de'],
      revision: 0,
    );
    final catalog = Revision3DialogLineEntryCatalog.fromContentIndex(
      _contentIndex(extraEntities: <Map<String, Object?>>[collision]),
    );

    final first = Revision3DialogLineEntryTechnicalPlan.forCheckpoint(
      catalog: catalog,
      input: input,
    );
    final second = Revision3DialogLineEntryTechnicalPlan.forCheckpoint(
      catalog: catalog,
      input: input,
    );

    expect(first.lineId, _generatedId('line', seed, 1));
    expect(second.lineId, first.lineId);
    expect(
      second.localization.localizationId,
      first.localization.localizationId,
    );
    expect(second.voiceSlot?.slotId, first.voiceSlot?.slotId);
    final generated = <String>{
      first.lineId,
      first.localization.localizationId,
      first.voiceSlot!.slotId,
    };
    expect(generated, hasLength(3));
    expect(generated.intersection(catalog.entityIds), isEmpty);
    expect(
      authoringRevision3VoiceArchiveBasenameStemIsSafe(
        (first.localization
                as AuthoringRevision3DialogLocalizationCreateIntentV1)
            .locId,
      ),
      isTrue,
    );
  });

  test(
    'technical planning retries case-folded runtime identity collisions',
    () {
      final input = _createInput();
      final seed = _planSeed(input, 7);
      final occupiedLineCandidate = _generatedId('line', seed, 0);
      final occupiedLocalizationCandidate = _generatedId(
        'localization',
        seed,
        0,
      );
      const foreignLineEntityId = '55555555555555555555555555555555';
      const foreignLocalizationEntityId = '66666666666666666666666666666666';
      final catalog = Revision3DialogLineEntryCatalog.fromContentIndex(
        _contentIndex(
          extraEntities: <Map<String, Object?>>[
            _localizationEntity(
              id: foreignLineEntityId,
              displayName: 'Runtime line identity collision',
              locId: 'gore_dialog_${occupiedLineCandidate.toUpperCase()}',
              locales: const <String>['de'],
              revision: 0,
            ),
            _localizationEntity(
              id: foreignLocalizationEntityId,
              displayName: 'Runtime localization identity collision',
              locId: 'gore_${occupiedLocalizationCandidate.toUpperCase()}',
              locales: const <String>['de'],
              revision: 0,
            ),
          ],
        ),
      );

      expect(occupiedLineCandidate, isNot(foreignLineEntityId));
      expect(occupiedLocalizationCandidate, isNot(foreignLocalizationEntityId));
      expect(
        catalog.primaryIdentitiesFolded,
        contains('gore_dialog_$occupiedLineCandidate'),
      );
      expect(
        catalog.primaryIdentitiesFolded,
        contains('gore_$occupiedLocalizationCandidate'),
      );
      expect(
        () => catalog.primaryIdentitiesFolded.add('gore_forbidden'),
        throwsUnsupportedError,
      );

      final first = Revision3DialogLineEntryTechnicalPlan.forCheckpoint(
        catalog: catalog,
        input: input,
      );
      final second = Revision3DialogLineEntryTechnicalPlan.forCheckpoint(
        catalog: catalog,
        input: input,
      );
      final localization =
          first.localization
              as AuthoringRevision3DialogLocalizationCreateIntentV1;

      expect(first.lineId, _generatedId('line', seed, 1));
      expect(
        localization.localizationId,
        _generatedId('localization', seed, 1),
      );
      expect(
        first.lineAuthoredIdentity,
        isNot(equalsIgnoringCase('gore_dialog_$occupiedLineCandidate')),
      );
      expect(
        localization.locId,
        isNot(equalsIgnoringCase('gore_$occupiedLocalizationCandidate')),
      );
      expect(second.lineId, first.lineId);
      expect(second.localization.localizationId, localization.localizationId);
    },
  );

  test('generated display names remain inside the native 256-byte cap', () {
    final catalog = Revision3DialogLineEntryCatalog.fromContentIndex(
      _contentIndex(),
    );
    final input = _createInput(name: 'x' * 192);
    final plan = Revision3DialogLineEntryTechnicalPlan.forCheckpoint(
      catalog: catalog,
      input: input,
    );
    final localization =
        plan.localization as AuthoringRevision3DialogLocalizationCreateIntentV1;

    expect(
      utf8.encode(localization.displayName).length,
      lessThanOrEqualTo(256),
    );
    expect(
      utf8.encode(plan.voiceSlot!.displayName).length,
      lessThanOrEqualTo(256),
    );
    expect(() => _createInput(name: 'x' * 193), throwsFormatException);
  });

  test('ReuseExact plan binds the fresh choice revision and identity', () {
    final catalog = Revision3DialogLineEntryCatalog.fromContentIndex(
      _contentIndex(),
    );
    final input = Revision3DialogLineEntryInput.reuseExact(
      lineDisplayName: 'Reuse greeting',
      locale: 'de',
      localizationId: _unownedLocalizationId,
      createVoiceSlot: false,
    );

    final plan = Revision3DialogLineEntryTechnicalPlan.forCheckpoint(
      catalog: catalog,
      input: input,
    );
    final reuse =
        plan.localization
            as AuthoringRevision3DialogLocalizationReuseExactIntentV1;
    expect(reuse.localizationId, _unownedLocalizationId);
    expect(reuse.expectedLocalizationRevision, 4);
    expect(reuse.expectedLocId, 'GORE_REUSABLE_GREETING');
    expect(plan.voiceSlot, isNull);
  });

  test('service maps an exact read to an ID-free friendly preview', () async {
    final catalog = Revision3DialogLineEntryCatalog.fromContentIndex(
      _contentIndex(),
    );
    var reads = 0;
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async {
            reads++;
            expect(expectedProjectId, _projectId);
            expect(expectedProjectRevision, 7);
            expect(localizationId, _unownedLocalizationId);
            expect(expectedLocalizationRevision, 4);
            expect(expectedLocId, 'GORE_REUSABLE_GREETING');
            return _successfulLocalizationRead(
              expectedProjectId: expectedProjectId,
              expectedProjectRevision: expectedProjectRevision,
              localizationId: localizationId,
              expectedLocalizationRevision: expectedLocalizationRevision,
              expectedLocId: expectedLocId,
            );
          },
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw StateError('preview must not publish'),
    );

    final preview = await service.loadReusableLocalizationPreview(
      checkpoint: catalog,
      localizationId: _unownedLocalizationId,
    );

    expect(reads, 1);
    expect(preview.authorableLocales, <String>['de', 'en']);
    expect(preview.locale('de')?.text, 'Bleib stehen!');
    expect(preview.locale('de')?.truncated, isFalse);
    expect(
      preview.toString(),
      isNot(anyOf(contains(_unownedLocalizationId), contains('GORE_'))),
    );
  });

  test('service rejects an exact preview that changed checkpoint', () async {
    final catalog = Revision3DialogLineEntryCatalog.fromContentIndex(
      _contentIndex(),
    );
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async => _localizationReadResult(
            expectedProjectId: expectedProjectId,
            expectedProjectRevision: expectedProjectRevision,
            localizationId: localizationId,
            expectedLocalizationRevision: expectedLocalizationRevision,
            expectedLocId: expectedLocId,
            actualProjectRevision: expectedProjectRevision + 1,
          ),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw StateError('stale preview must not publish'),
    );

    await expectLater(
      service.loadReusableLocalizationPreview(
        checkpoint: catalog,
        localizationId: _unownedLocalizationId,
      ),
      throwsA(isA<Revision3DialogLineEntryStaleCheckpointException>()),
    );
  });

  test('ReuseExact re-reads and rejects whitespace-only locale text', () async {
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => _contentIndex(),
      readExactLocalization:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required localizationId,
            required expectedLocalizationRevision,
            required expectedLocId,
          }) async => _localizationReadResult(
            expectedProjectId: expectedProjectId,
            expectedProjectRevision: expectedProjectRevision,
            localizationId: localizationId,
            expectedLocalizationRevision: expectedLocalizationRevision,
            expectedLocId: expectedLocId,
            locales: <Map<String, Object?>>[
              <String, Object?>{
                'locale': 'de',
                'preview': '   ',
                'truncated': false,
                'has_nonempty_text': false,
              },
              <String, Object?>{
                'locale': 'en',
                'preview': 'Stop there!',
                'truncated': false,
                'has_nonempty_text': true,
              },
            ],
          ),
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async => throw StateError('empty locale must not publish'),
    );
    final checkpoint = await service.loadCatalog();

    await expectLater(
      service.publish(
        checkpoint: checkpoint,
        input: Revision3DialogLineEntryInput.reuseExact(
          lineDisplayName: 'Empty German line',
          locale: 'de',
          localizationId: _unownedLocalizationId,
        ),
      ),
      throwsA(isA<Revision3DialogLineEntryNoReusableTextException>()),
    );
  });

  test(
    'service reopens, CAS-checks, publishes, and verifies its receipt',
    () async {
      final index = _contentIndex();
      var loads = 0;
      var exactReads = 0;
      var publications = 0;
      late Revision3DialogLineEntryTechnicalPlan publishedPlan;
      final service = Revision3DialogLineEntryAuthoringService(
        loadContentIndex: () async {
          loads++;
          return index;
        },
        readExactLocalization:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required localizationId,
              required expectedLocalizationRevision,
              required expectedLocId,
            }) async {
              exactReads++;
              return _successfulLocalizationRead(
                expectedProjectId: expectedProjectId,
                expectedProjectRevision: expectedProjectRevision,
                localizationId: localizationId,
                expectedLocalizationRevision: expectedLocalizationRevision,
                expectedLocId: expectedLocId,
              );
            },
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async {
              publications++;
              publishedPlan = plan;
              expect(expectedProjectId, _projectId);
              expect(expectedProjectRevision, 7);
              return Revision3DialogLineEntryPublication(
                projectId: expectedProjectId,
                projectRevision: expectedProjectRevision + 1,
                lineId: plan.lineId,
                localizationId: plan.localization.localizationId,
                localizationAction:
                    AuthoringRevision3DialogLocalizationAction.created,
                voiceSlotId: plan.voiceSlot?.slotId,
                locale: plan.locale,
              );
            },
      );
      final checkpoint = await service.loadCatalog();
      final publication = await service.publish(
        checkpoint: checkpoint,
        input: _createInput(),
      );

      expect(loads, 2);
      expect(exactReads, 0, reason: 'Create must not read existing text');
      expect(publications, 1);
      expect(publication.lineId, publishedPlan.lineId);
      expect(publication.projectRevision, 8);
    },
  );

  test('service rejects stale checkpoints before publication', () async {
    final indices = <Revision3ContentIndex>[
      _contentIndex(),
      _contentIndex(revision: 8),
    ];
    var publications = 0;
    final service = Revision3DialogLineEntryAuthoringService(
      loadContentIndex: () async => indices.removeAt(0),
      readExactLocalization: _successfulLocalizationRead,
      publishTechnicalPlan:
          ({
            required expectedProjectId,
            required expectedProjectRevision,
            required plan,
          }) async {
            publications++;
            throw StateError('must not publish stale dialog plan');
          },
    );
    final checkpoint = await service.loadCatalog();

    await expectLater(
      service.publish(checkpoint: checkpoint, input: _createInput()),
      throwsA(isA<Revision3DialogLineEntryStaleCheckpointException>()),
    );
    expect(publications, 0);
  });

  test(
    'service maps reopen and rejects a mismatched publication receipt',
    () async {
      final reopen = Revision3DialogLineEntryAuthoringService(
        loadContentIndex: () async {
          throw const Revision3ContentRequiresReopenException();
        },
        readExactLocalization: _successfulLocalizationRead,
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async => throw StateError('unreachable'),
      );
      await expectLater(
        reopen.loadCatalog(),
        throwsA(isA<Revision3DialogLineEntryRequiresReopenException>()),
      );

      final index = _contentIndex();
      final mismatch = Revision3DialogLineEntryAuthoringService(
        loadContentIndex: () async => index,
        readExactLocalization: _successfulLocalizationRead,
        publishTechnicalPlan:
            ({
              required expectedProjectId,
              required expectedProjectRevision,
              required plan,
            }) async => Revision3DialogLineEntryPublication(
              projectId: expectedProjectId,
              projectRevision: expectedProjectRevision + 1,
              lineId: _ownerLineId,
              localizationId: plan.localization.localizationId,
              localizationAction:
                  AuthoringRevision3DialogLocalizationAction.created,
              voiceSlotId: plan.voiceSlot?.slotId,
              locale: plan.locale,
            ),
      );
      final checkpoint = await mismatch.loadCatalog();
      await expectLater(
        mismatch.publish(checkpoint: checkpoint, input: _createInput()),
        throwsA(isA<Revision3DialogLineEntryRequiresReopenException>()),
      );
    },
  );
}
