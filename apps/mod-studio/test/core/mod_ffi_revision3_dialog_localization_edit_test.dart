import 'dart:collection';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

const _readCommand =
    'authoring_store_read_revision3_dialog_localization_edit_seed_v1';
const _prepareCommand =
    'authoring_store_prepare_revision3_dialog_localization_edit_v1';
const _projectId = '11111111111111111111111111111111';
const _localizationId = '22222222222222222222222222222222';
const _lineId = '33333333333333333333333333333333';
const _locId = 'GORE_ASGHAN_WARNING';

AuthoringWorkingHead _head(String digit) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{'byte_len': 321, 'sha256': digit * 64},
      }),
    );

String _projectJson({
  int revision = 7,
  int localizationRevision = 4,
}) => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 3,
  'project_id': _projectId,
  'revision': revision,
  'meta': <String, Object?>{
    'name': 'Localization edit fixture',
    'version': '1.0.0',
    'author': 'tests',
  },
  'target': <String, Object?>{
    'executable': <String, Object?>{'byte_len': 171698176, 'sha256': 'a' * 64},
  },
  'authoring_locales': <Object?>['de', 'en'],
  'entities': SplayTreeMap<String, Object?>.from(<String, Object?>{
    _localizationId: <String, Object?>{
      'id': _localizationId,
      'display_name': 'Asghan warning',
      'origin': <String, Object?>{'type': 'new', 'authored_runtime_id': _locId},
      'revision': localizationRevision,
      'payload': <String, Object?>{
        'kind': 'localization_entry',
        'data': <String, Object?>{
          'loc_id': _locId,
          'texts': SplayTreeMap<String, Object?>.from(<String, Object?>{
            'de': 'Bleib stehen!',
            'en': 'Stop right there!',
          }),
        },
      },
    },
  }),
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});

Map<String, Object?> _seedResponse({
  String? headJson,
  String projectId = _projectId,
  int projectRevision = 7,
  String localizationId = _localizationId,
  int localizationRevision = 4,
  String locId = _locId,
}) => <String, Object?>{
  'ok': true,
  'outcome': 'read_only',
  'head_json': headJson ?? _head('b').canonicalJson,
  'project_id': projectId,
  'project_revision': projectRevision,
  'localization_id': localizationId,
  'localization_revision': localizationRevision,
  'loc_id': locId,
  'locales': <Object?>[
    <String, Object?>{
      'locale': 'de',
      'text': 'Bleib stehen!',
      'voice_slot_present': true,
      'candidate_count': 1,
    },
    <String, Object?>{
      'locale': 'en',
      'text': 'Stop right there!',
      'voice_slot_present': false,
      'candidate_count': 0,
    },
  ],
  'line_backlinks': <Object?>[
    <String, Object?>{
      'line_id': _lineId,
      'line_revision': 2,
      'display_name': 'Asghan warning line',
      'speaker_hint': 'Asghan',
      'voice_slot_locales': <Object?>['de'],
    },
  ],
  'content_authority': 'read_only_exact_current_localization_edit_seed',
  'build_status': 'not_evaluated',
  'runtime_status': 'runtime_unqualified',
  'publication_status': 'not_applicable',
};

String _candidateProjectJson({
  required String basisProjectJson,
  required Map<String, String> texts,
}) {
  final candidate = (jsonDecode(basisProjectJson) as Map)
      .cast<String, Object?>();
  candidate['revision'] = 8;
  candidate['authoring_locales'] = <Object?>['de', 'en', 'fr'];
  final entities = (candidate['entities']! as Map).cast<String, Object?>();
  final localization = (entities[_localizationId]! as Map)
      .cast<String, Object?>();
  localization['revision'] = 5;
  final payload = (localization['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  data['texts'] = SplayTreeMap<String, Object?>.from(texts);
  return jsonEncode(candidate);
}

Map<String, Object?> _prepareResponse({
  required String basisProjectJson,
  required Map<String, String> texts,
  List<String> addedLocales = const <String>['fr'],
  List<String> removedLocales = const <String>[],
  String buildStatus = 'blocked',
}) => <String, Object?>{
  'ok': true,
  'outcome': 'prepared_unpublished',
  'basis_head_json': _head('b').canonicalJson,
  'head_json': _head('c').canonicalJson,
  'project_json': _candidateProjectJson(
    basisProjectJson: basisProjectJson,
    texts: texts,
  ),
  'project_id': _projectId,
  'revision': 8,
  'localization_id': _localizationId,
  'localization_revision': 5,
  'added_locales': addedLocales,
  'removed_locales': removedLocales,
  'build_status': buildStatus,
  'runtime_status': 'runtime_unqualified',
  'topic_authority': 'not_granted',
  'publication_status': 'not_supported',
};

Map<String, Object?> _clone(Map<String, Object?> value) =>
    (jsonDecode(jsonEncode(value)) as Map).cast<String, Object?>();

Matcher get _throwsMalformed => throwsA(
  isA<ModFfiException>().having(
    (error) => error.code,
    'code',
    ModFfiException.malformedNativeResponseCode,
  ),
);

void main() {
  test(
    'seed read uses the closed project-only wire and exact Voice facts',
    () async {
      final expectedHead = _head('b');
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          _readCommand: _seedResponse(),
        },
      );
      final result = await ModFfi(core)
          .authoringStoreReadRevision3DialogLocalizationEditSeedV1(
            root: r'C:\Mods\Dialog.goreproj',
            expectedHead: expectedHead,
            localizationId: _localizationId,
            expectedLocalizationRevision: 4,
            expectedLocId: _locId,
          );

      expect(core.calls.single.command, _readCommand);
      expect(core.calls.single.payload.keys, <String>[
        'root',
        'expected_head_json',
        'localization_id',
        'expected_localization_revision',
        'expected_loc_id',
      ]);
      expect(core.calls.single.payload, isNot(contains('game_root')));
      expect(
        core.calls.single.payload,
        isNot(contains('current_project_json')),
      );
      expect(result.locales.map((locale) => locale.locale), <String>[
        'de',
        'en',
      ]);
      expect(result.locales.first.voiceSlotPresent, isTrue);
      expect(result.locales.first.candidateCount, 1);
      expect(result.lineBacklinks.single.displayName, 'Asghan warning line');
      expect(result.lineBacklinks.single.speakerHint, 'Asghan');
      expect(result.lineBacklinks.single.voiceSlotLocales, <String>['de']);
      expect(
        requiredStudioCoreCommands,
        containsAll(<String>[_readCommand, _prepareCommand]),
      );
      expect(
        requiredStudioCoreCommands,
        orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
      );
    },
  );

  test(
    'seed response identity, order, Voice facts, and authority fail closed',
    () async {
      Future<void> reject(Map<String, Object?> response) async {
        final core = FakeGoreCoreFfiService(
          responses: <String, Map<String, Object?>>{_readCommand: response},
        );
        await expectLater(
          ModFfi(core).authoringStoreReadRevision3DialogLocalizationEditSeedV1(
            root: r'C:\Mods\Dialog.goreproj',
            expectedHead: _head('b'),
            localizationId: _localizationId,
            expectedLocalizationRevision: 4,
            expectedLocId: _locId,
          ),
          _throwsMalformed,
        );
      }

      await reject(_seedResponse(localizationRevision: 5));
      final unordered = _clone(_seedResponse());
      (unordered['locales']! as List<Object?>).setAll(
        0,
        (unordered['locales']! as List<Object?>).reversed,
      );
      await reject(unordered);
      final falseVoice = _clone(_seedResponse());
      ((falseVoice['line_backlinks']! as List<Object?>).single!
              as Map<String, Object?>)['voice_slot_locales'] =
          <Object?>[];
      await reject(falseVoice);
      final impossibleCandidates = _clone(_seedResponse());
      ((impossibleCandidates['locales']! as List<Object?>).last!
              as Map<String, Object?>)['candidate_count'] =
          1;
      await reject(impossibleCandidates);
      final authority = _clone(_seedResponse());
      authority['content_authority'] = 'editable';
      await reject(authority);
    },
  );

  test(
    'prepare carries canonical exact request and accepts only exact delta',
    () async {
      final basisProjectJson = _projectJson();
      final texts = SplayTreeMap<String, String>.from(<String, String>{
        'de': 'Bleib stehen!',
        'en': 'Stop now!',
        'fr': 'Halte-là!',
      });
      final request =
          AuthoringRevision3DialogLocalizationEditRequestV1.forProject(
            expectedHead: _head('b'),
            currentProjectJson: basisProjectJson,
            localizationId: _localizationId,
            expectedLocalizationRevision: 4,
            expectedLocId: _locId,
            texts: texts,
          );
      final decoded = (jsonDecode(request.canonicalJson) as Map)
          .cast<String, Object?>();
      expect(decoded.keys, <String>[
        'expected_head',
        'expected_project_id',
        'expected_revision',
        'expected_target',
        'localization_id',
        'expected_localization_revision',
        'expected_loc_id',
        'texts',
      ]);
      expect((decoded['texts']! as Map).keys, <String>['de', 'en', 'fr']);

      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          _prepareCommand: _prepareResponse(
            basisProjectJson: basisProjectJson,
            texts: texts,
          ),
        },
      );
      final result = await ModFfi(core)
          .authoringStorePrepareRevision3DialogLocalizationEditV1(
            root: r'C:\Mods\Dialog.goreproj',
            currentProjectJson: basisProjectJson,
            request: request,
          );
      expect(core.calls.single.command, _prepareCommand);
      expect(core.calls.single.payload.keys, <String>[
        'current_project_json',
        'localization_edit_request_json',
        'root',
      ]);
      expect(core.calls.single.payload, isNot(contains('game_root')));
      expect(result.revision, 8);
      expect(result.localizationRevision, 5);
      expect(result.addedLocales, <String>['fr']);
      expect(result.removedLocales, isEmpty);
      expect(
        result.buildStatus,
        AuthoringRevision3DialogLocalizationEditBuildStatus.blocked,
      );
      expect(
        result.publicationStatus,
        AuthoringRevision3DialogLocalizationEditPublicationStatus.notSupported,
      );
    },
  );

  test(
    'no-op, blank-only, NUL, and unsafe response deltas are rejected',
    () async {
      final basisProjectJson = _projectJson();
      expect(
        () => AuthoringRevision3DialogLocalizationEditRequestV1.forProject(
          expectedHead: _head('b'),
          currentProjectJson: basisProjectJson,
          localizationId: _localizationId,
          expectedLocalizationRevision: 4,
          expectedLocId: _locId,
          texts: const <String, String>{
            'de': 'Bleib stehen!',
            'en': 'Stop right there!',
          },
        ),
        throwsFormatException,
      );
      expect(
        () => AuthoringRevision3DialogLocalizationEditRequestV1.forProject(
          expectedHead: _head('b'),
          currentProjectJson: basisProjectJson,
          localizationId: _localizationId,
          expectedLocalizationRevision: 4,
          expectedLocId: _locId,
          texts: const <String, String>{'de': ' ', 'en': '\t'},
        ),
        throwsFormatException,
      );
      expect(
        () => AuthoringRevision3DialogLocalizationEditRequestV1.forProject(
          expectedHead: _head('b'),
          currentProjectJson: basisProjectJson,
          localizationId: _localizationId,
          expectedLocalizationRevision: 4,
          expectedLocId: _locId,
          texts: const <String, String>{'de': 'Neu\u0000'},
        ),
        throwsFormatException,
      );

      final texts = SplayTreeMap<String, String>.from(<String, String>{
        'de': 'Bleib stehen!',
        'en': 'Stop now!',
        'fr': 'Halte-là!',
      });
      final request =
          AuthoringRevision3DialogLocalizationEditRequestV1.forProject(
            expectedHead: _head('b'),
            currentProjectJson: basisProjectJson,
            localizationId: _localizationId,
            expectedLocalizationRevision: 4,
            expectedLocId: _locId,
            texts: texts,
          );

      Future<void> reject(Map<String, Object?> response) async {
        final core = FakeGoreCoreFfiService(
          responses: <String, Map<String, Object?>>{_prepareCommand: response},
        );
        await expectLater(
          ModFfi(core).authoringStorePrepareRevision3DialogLocalizationEditV1(
            root: r'C:\Mods\Dialog.goreproj',
            currentProjectJson: basisProjectJson,
            request: request,
          ),
          _throwsMalformed,
        );
      }

      await reject(
        _prepareResponse(
          basisProjectJson: basisProjectJson,
          texts: texts,
          addedLocales: const <String>[],
        ),
      );
      await reject(
        _prepareResponse(
          basisProjectJson: basisProjectJson,
          texts: texts,
          buildStatus: 'ready',
        ),
      );
      final tampered = _prepareResponse(
        basisProjectJson: basisProjectJson,
        texts: texts,
      );
      final candidate = jsonDecode(tampered['project_json']! as String) as Map;
      (candidate['meta'] as Map)['name'] = 'Tampered';
      tampered['project_json'] = jsonEncode(candidate);
      await reject(tampered);
    },
  );
}
