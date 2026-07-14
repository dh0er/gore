import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_voice_fixture.dart';

const _gameRoot = r'D:\Games\Gothic Remake';

String _headJson(String byte) => jsonEncode(<String, Object?>{
  'store_format': 1,
  'snapshot': <String, Object?>{
    'byte_len': 321,
    'sha256': List<String>.filled(64, byte).join(),
  },
});

AuthoringRevision3VoiceTakeRequestV1 _request({
  AuthoringRevision3VoiceTakeStatus status =
      AuthoringRevision3VoiceTakeStatus.recorded,
  bool selectTake = false,
  String? text,
}) => AuthoringRevision3VoiceTakeRequestV1.forProject(
  expectedHead: AuthoringWorkingHead.fromCanonicalJson(_headJson('b')),
  currentProjectJson: revision3VoiceFixtureProjectJson(),
  lineId: revision3VoiceFixtureLineId,
  slotId: revision3VoiceFixtureSlotId,
  takeId: revision3VoiceFixtureTakeId,
  locale: 'de',
  text: text,
  takeDisplayName: 'Asghan DE Take 1',
  logicalName: 'GRD_263_ASGHAN_OPEN_INFO_06_02.ogg',
  status: status,
  selectTake: selectTake,
);

Revision3VoiceFixture _fixture({
  AuthoringRevision3VoiceTakeRequestV1? request,
}) {
  final exactRequest = request ?? _request();
  return Revision3VoiceFixture.fromBasis(
    basisHead: exactRequest.expectedHead,
    basisProjectJson: revision3VoiceFixtureProjectJson(),
    request: exactRequest,
  );
}

void main() {
  test('required command handshake includes native R3 Voice preparation', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_prepare_revision3_voice_take_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('Voice request is exact, project-bound, bounded, and status-safe', () {
    final request = _request();
    final reopened = AuthoringRevision3VoiceTakeRequestV1.fromCanonicalJson(
      request.canonicalJson,
    );
    expect(reopened.expectedHead.canonicalJson, _headJson('b'));
    expect(reopened.expectedProjectId, revision3VoiceFixtureProjectId);
    expect(reopened.expectedRevision, 7);
    expect(reopened.lineId, revision3VoiceFixtureLineId);
    expect(reopened.slotId, revision3VoiceFixtureSlotId);
    expect(reopened.takeId, revision3VoiceFixtureTakeId);
    expect(reopened.locale, 'de');
    expect(reopened.text, isNull);
    expect(reopened.canonicalJson, isNot(contains('"text"')));
    expect(reopened.status, AuthoringRevision3VoiceTakeStatus.recorded);
    expect(reopened.selectTake, isFalse);

    final raw = (jsonDecode(request.canonicalJson) as Map)
        .cast<String, Object?>();
    final reordered = <String, Object?>{
      'expected_project_id': raw['expected_project_id'],
      for (final entry in raw.entries)
        if (entry.key != 'expected_project_id') entry.key: entry.value,
    };
    final malformed = <String>[
      ' ${request.canonicalJson}',
      '${request.canonicalJson}\n',
      request.canonicalJson.replaceFirst(
        '"expected_revision":7',
        '"expected_revision":7,"expected_revision":7',
      ),
      jsonEncode(<String, Object?>{...raw, 'target_authority': 'granted'}),
      jsonEncode(<String, Object?>{...raw}..remove('expected_target')),
      jsonEncode(<String, Object?>{
        ...raw,
        'slot_id': revision3VoiceFixtureTakeId,
      }),
      jsonEncode(<String, Object?>{...raw, 'locale': 'de-de'}),
      jsonEncode(<String, Object?>{...raw, 'logical_name': 'voice.wav'}),
      for (final logicalName in <String>[
        '../x.ogg',
        r'dir\x.ogg',
        'C:x.ogg',
        'CON.ogg',
        'Lpt1.OGG',
        ' x.ogg',
        'x.ogg ',
        '.ogg',
        'x?.ogg',
      ])
        jsonEncode(<String, Object?>{...raw, 'logical_name': logicalName}),
      jsonEncode(<String, Object?>{
        ...raw,
        'status': 'reviewed',
        'select_take': true,
      }),
      jsonEncode(reordered),
    ];
    for (final value in malformed) {
      expect(
        () => AuthoringRevision3VoiceTakeRequestV1.fromCanonicalJson(value),
        throwsFormatException,
      );
    }

    final approved = _request(
      status: AuthoringRevision3VoiceTakeStatus.approved,
      selectTake: true,
    );
    expect(approved.selectTake, isTrue);
  });

  test('strict response accepts only the exact unresolved Voice delta', () {
    final request = _request();
    final fixture = _fixture(request: request);
    final prepared = AuthoringRevision3VoiceTakePreparation.fromJson(
      fixture.response(),
      currentProjectJson: revision3VoiceFixtureProjectJson(),
      request: request,
    );
    expect(prepared.projectId, revision3VoiceFixtureProjectId);
    expect(prepared.revision, 8);
    expect(prepared.localizationId, revision3VoiceFixtureLocalizationId);
    expect(prepared.slotCreated, isTrue);
    expect(prepared.selected, isFalse);
    expect(prepared.asset.sha256, revision3VoiceFixtureAssetSha256);
    expect(prepared.asset.byteLength, 100);
    expect(prepared.ogg.codec, AuthoringRevision3VoiceOggCodec.vorbis);
    expect(prepared.buildStatus, AuthoringRevision3VoiceBuildStatus.blocked);
    expect(
      prepared.runtimeStatus,
      AuthoringRevision3VoiceRuntimeStatus.runtimeUnqualified,
    );
    expect(
      prepared.targetAuthority,
      AuthoringRevision3VoiceTargetAuthority.notGranted,
    );
    expect(
      prepared.publicationStatus,
      AuthoringRevision3VoiceNativePublicationStatus.notSupported,
    );
    final basis = (jsonDecode(revision3VoiceFixtureProjectJson()) as Map)
        .cast<String, Object?>();
    final candidate = (jsonDecode(prepared.projectJson) as Map)
        .cast<String, Object?>();
    final basisEntities = (basis['entities']! as Map).cast<String, Object?>();
    final candidateEntities = (candidate['entities']! as Map)
        .cast<String, Object?>();
    expect(
      candidateEntities[revision3VoiceFixtureLocalizationId],
      basisEntities[revision3VoiceFixtureLocalizationId],
      reason: 'voice-only import must preserve LocalizationEntry bytes/value',
    );
  });

  test('explicit dialog text edit changes localization exactly once', () {
    final request = _request(text: 'Du willst in die Mine?');
    final fixture = _fixture(request: request);
    final prepared = AuthoringRevision3VoiceTakePreparation.fromJson(
      fixture.response(),
      currentProjectJson: revision3VoiceFixtureProjectJson(),
      request: request,
    );
    final project = (jsonDecode(prepared.projectJson) as Map)
        .cast<String, Object?>();
    final entities = (project['entities']! as Map).cast<String, Object?>();
    final localization = (entities[revision3VoiceFixtureLocalizationId]! as Map)
        .cast<String, Object?>();
    expect(localization['revision'], 5);
    final payload = (localization['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();
    expect(data['texts'], <String, Object?>{'de': 'Du willst in die Mine?'});
  });

  test('response rejects authority escalation and any non-exact delta', () {
    final request = _request();
    final mutations = <void Function(Map<String, Object?>)>[
      (response) => response['outcome'] = 'published',
      (response) => response['runtime_status'] = 'qualified',
      (response) => response['target_authority'] = 'granted',
      (response) => response['publication_status'] = 'published',
      (response) => response['selected'] = true,
      (response) => response['localization_id'] = revision3VoiceFixtureLineId,
      (response) => response['asset'] = <String, Object?>{
        ...(response['asset']! as Map).cast<String, Object?>(),
        'byte_len': 101,
      },
      (response) => response['project_json'] = _mutateCandidate(
        response,
        (project) => project['meta'] = <String, Object?>{
          'name': 'forged',
          'version': '1.0.0',
          'author': 'tests',
        },
      ),
      (response) => response['project_json'] = _mutateCandidate(response, (
        project,
      ) {
        final entities = (project['entities']! as Map).cast<String, Object?>();
        final slot = (entities[revision3VoiceFixtureSlotId]! as Map)
            .cast<String, Object?>();
        final payload = (slot['payload']! as Map).cast<String, Object?>();
        final data = (payload['data']! as Map).cast<String, Object?>();
        data['target_resolution'] = <String, Object?>{
          'state': 'resolved',
          'target': <String, Object?>{},
        };
      }),
      (response) => response['project_json'] = _mutateCandidate(response, (
        project,
      ) {
        final entities = (project['entities']! as Map).cast<String, Object?>();
        final take = (entities[revision3VoiceFixtureTakeId]! as Map)
            .cast<String, Object?>();
        final origin = (take['origin']! as Map).cast<String, Object?>();
        origin['importer'] = 'forged';
      }),
    ];
    for (final mutate in mutations) {
      final response = _fixture(request: request).response();
      mutate(response);
      expect(
        () => AuthoringRevision3VoiceTakePreparation.fromJson(
          response,
          currentProjectJson: revision3VoiceFixtureProjectJson(),
          request: request,
        ),
        throwsFormatException,
      );
    }
  });

  test(
    'wrapper sends exact canonical payload and normalizes bad response',
    () async {
      final request = _request();
      final fixture = _fixture(request: request);
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_voice_take_v1': fixture.response(),
        },
      );
      final prepared = await ModFfi(core)
          .authoringStorePrepareRevision3VoiceTakeV1(
            root: r'C:\Mods\Voice.goreproj',
            gameRoot: _gameRoot,
            source: r'D:\Recordings\asghan.ogg',
            currentProjectJson: revision3VoiceFixtureProjectJson(),
            request: request,
          );
      expect(prepared.takeId, revision3VoiceFixtureTakeId);
      expect(core.calls, hasLength(1));
      expect(core.calls.single.payload, <String, Object?>{
        'current_project_json': revision3VoiceFixtureProjectJson(),
        'game_root': _gameRoot,
        'root': r'C:\Mods\Voice.goreproj',
        'source': r'D:\Recordings\asghan.ogg',
        'voice_request_json': request.canonicalJson,
      });

      final badCore = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_voice_take_v1': fixture.response()
            ..['target_authority'] = 'granted',
        },
      );
      await expectLater(
        ModFfi(badCore).authoringStorePrepareRevision3VoiceTakeV1(
          root: 'root',
          gameRoot: _gameRoot,
          source: 'take.ogg',
          currentProjectJson: revision3VoiceFixtureProjectJson(),
          request: request,
        ),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            ModFfiException.malformedNativeResponseCode,
          ),
        ),
      );
    },
  );
}

String _mutateCandidate(
  Map<String, Object?> response,
  void Function(Map<String, Object?> project) mutate,
) {
  final project = (jsonDecode(response['project_json']! as String) as Map)
      .cast<String, Object?>();
  mutate(project);
  return jsonEncode(project);
}
