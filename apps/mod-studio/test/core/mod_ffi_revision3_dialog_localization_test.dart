import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

const _command = 'authoring_store_read_revision3_dialog_localization_v1';
const _projectId = '11111111111111111111111111111111';
const _localizationId = '22222222222222222222222222222222';
const _locId = 'GORE_EXISTING_TEXT';

AuthoringWorkingHead _head(String digit) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{'byte_len': 321, 'sha256': digit * 64},
      }),
    );

Map<String, Object?> _response({
  AuthoringWorkingHead? head,
  String projectId = _projectId,
  int projectRevision = 7,
  String localizationId = _localizationId,
  int localizationRevision = 4,
  String locId = _locId,
}) => <String, Object?>{
  'ok': true,
  'outcome': 'read_only',
  'head_json': (head ?? _head('a')).canonicalJson,
  'project_id': projectId,
  'project_revision': projectRevision,
  'localization_id': localizationId,
  'localization_revision': localizationRevision,
  'loc_id': locId,
  'locales': <Object?>[
    <String, Object?>{
      'locale': 'de',
      'preview': 'Grüße aus der Mine 👋',
      'truncated': false,
      'has_nonempty_text': true,
    },
    <String, Object?>{
      'locale': 'en',
      'preview': '   ',
      'truncated': false,
      'has_nonempty_text': false,
    },
  ],
  'content_authority': 'read_only_exact_current_localization',
  'build_status': 'not_evaluated',
  'runtime_status': 'runtime_unqualified',
  'publication_status': 'not_applicable',
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

Future<AuthoringRevision3DialogLocalizationReadResult> _read(
  Map<String, Object?> response, {
  void Function(FakeGoreCoreFfiService core)? inspect,
}) async {
  final core = FakeGoreCoreFfiService(
    responses: <String, Map<String, Object?>>{_command: response},
  );
  final result = await ModFfi(core)
      .authoringStoreReadRevision3DialogLocalizationV1(
        root: r'C:\Mods\Dialog.goreproj',
        expectedHead: _head('a'),
        localizationId: _localizationId,
        expectedLocalizationRevision: 4,
        expectedLocId: _locId,
      );
  inspect?.call(core);
  return result;
}

void main() {
  test(
    'wire is project-only and parses closed UTF-8 preview authority',
    () async {
      final result = await _read(
        _response(),
        inspect: (core) {
          expect(core.calls.single.command, _command);
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
        },
      );

      expect(result.head.canonicalJson, _head('a').canonicalJson);
      expect(result.projectId, _projectId);
      expect(result.projectRevision, 7);
      expect(result.localizationId, _localizationId);
      expect(result.localizationRevision, 4);
      expect(result.locId, _locId);
      expect(result.locales.map((locale) => locale.locale), <String>[
        'de',
        'en',
      ]);
      expect(result.locales.first.preview, 'Grüße aus der Mine 👋');
      expect(result.locales.first.truncated, isFalse);
      expect(result.locales.first.hasNonemptyText, isTrue);
      expect(result.locales.last.hasNonemptyText, isFalse);
      expect(
        () => result.locales.add(result.locales.first),
        throwsUnsupportedError,
      );
      expect(
        result.contentAuthority,
        AuthoringRevision3DialogLocalizationReadContentAuthority
            .readOnlyExactCurrentLocalization,
      );
      expect(
        result.buildStatus,
        AuthoringRevision3DialogLocalizationReadBuildStatus.notEvaluated,
      );
      expect(
        result.runtimeStatus,
        AuthoringRevision3DialogLocalizationReadRuntimeStatus
            .runtimeUnqualified,
      );
      expect(
        result.publicationStatus,
        AuthoringRevision3DialogLocalizationReadPublicationStatus.notApplicable,
      );
      expect(requiredStudioCoreCommands, contains(_command));
      expect(
        requiredStudioCoreCommands,
        orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
      );
    },
  );

  test('candidate identity mismatches are rejected as malformed', () async {
    for (final mismatch in <Map<String, Object?>>[
      _response(localizationId: '33333333333333333333333333333333'),
      _response(localizationRevision: 5),
      _response(locId: 'GORE_CHANGED_TEXT'),
      _response(head: _head('b')),
    ]) {
      await expectLater(_read(mismatch), _throwsMalformed);
    }
  });

  test(
    'locale ordering, closed authority, and UTF-8 bounds fail closed',
    () async {
      final unordered = _clone(_response());
      (unordered['locales']! as List<Object?>).setAll(
        0,
        (unordered['locales']! as List<Object?>).reversed,
      );
      await expectLater(_read(unordered), _throwsMalformed);

      final authority = _clone(_response());
      authority['content_authority'] = 'editable';
      await expectLater(_read(authority), _throwsMalformed);

      final tooLong = _clone(_response());
      ((tooLong['locales']! as List<Object?>).first!
              as Map<String, Object?>)['preview'] =
          'ü' * 257;
      await expectLater(_read(tooLong), _throwsMalformed);

      final malformedUtf16 = _clone(_response());
      ((malformedUtf16['locales']! as List<Object?>).first!
              as Map<String, Object?>)['preview'] =
          '\uD800';
      await expectLater(_read(malformedUtf16), _throwsMalformed);

      final legacyDisplayName = _clone(_response());
      legacyDisplayName['display_name'] = 'must not be accepted';
      await expectLater(_read(legacyDisplayName), _throwsMalformed);
    },
  );

  test(
    'read request and response accept the native LocID and locale caps',
    () async {
      final longestLocId = 'L' * 1020;
      final request = AuthoringRevision3DialogLocalizationReadRequestV1(
        expectedHead: _head('a'),
        localizationId: _localizationId,
        expectedLocalizationRevision: 4,
        expectedLocId: longestLocId,
      );
      expect(request.expectedLocId, longestLocId);
      expect(
        () => AuthoringRevision3DialogLocalizationReadRequestV1(
          expectedHead: _head('a'),
          localizationId: _localizationId,
          expectedLocalizationRevision: 4,
          expectedLocId: '${longestLocId}L',
        ),
        throwsFormatException,
      );

      final atLimit = _response();
      atLimit['locales'] = <Object?>[
        for (var index = 0; index < 1000; index++)
          <String, Object?>{
            'locale': 'aa-${index.toString().padLeft(3, '0')}',
            'preview': '',
            'truncated': false,
            'has_nonempty_text': false,
          },
      ];
      expect((await _read(atLimit)).locales, hasLength(1000));

      final overLimit = _clone(atLimit);
      (overLimit['locales']! as List<Object?>).add(<String, Object?>{
        'locale': 'aa-1000',
        'preview': '',
        'truncated': false,
        'has_nonempty_text': false,
      });
      await expectLater(_read(overLimit), _throwsMalformed);
    },
  );

  test('preview flags mirror Rust whitespace semantics exactly', () async {
    final bom = _response();
    final bomLocale =
        (bom['locales']! as List<Object?>).first! as Map<String, Object?>;
    bomLocale['preview'] = '\ufeff';
    bomLocale['has_nonempty_text'] = true;
    expect((await _read(bom)).locales.first.hasNonemptyText, isTrue);

    final bomMismatch = _clone(bom);
    ((bomMismatch['locales']! as List<Object?>).first!
            as Map<String, Object?>)['has_nonempty_text'] =
        false;
    await expectLater(_read(bomMismatch), _throwsMalformed);

    final impossibleTruncated = _response();
    final impossibleLocale =
        (impossibleTruncated['locales']! as List<Object?>).first!
            as Map<String, Object?>;
    impossibleLocale['preview'] = 'visible';
    impossibleLocale['truncated'] = true;
    impossibleLocale['has_nonempty_text'] = false;
    await expectLater(_read(impossibleTruncated), _throwsMalformed);

    for (final hasNonempty in <bool>[false, true]) {
      final unknownSuffix = _response();
      final locale =
          (unknownSuffix['locales']! as List<Object?>).first!
              as Map<String, Object?>;
      locale['preview'] = '\u2003';
      locale['truncated'] = true;
      locale['has_nonempty_text'] = hasNonempty;
      expect(
        (await _read(unknownSuffix)).locales.first.hasNonemptyText,
        hasNonempty,
      );
    }
  });
}
