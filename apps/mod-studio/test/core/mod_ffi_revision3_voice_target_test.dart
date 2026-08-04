import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_voice_fixture.dart';

const _locId = 'GRD_263_ASGHAN_OPEN_INFO_06_02';
const _gameRoot = r'D:\Games\Gothic Remake';
const _storeRoot = r'C:\Projects\Voice.goreproj';
const _archive = 'german_new.zip';
const _archiveBytes = 4096;
const _archiveSha =
    'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc';

AuthoringWorkingHead _headFor(String projectJson) =>
    AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': utf8.encode(projectJson).length,
          'sha256': crypto.sha256.convert(utf8.encode(projectJson)).toString(),
        },
      }),
    );

({String projectJson, AuthoringWorkingHead head}) _voiceBasis({
  int candidateCount = 1,
  AuthoringRevision3VoiceTakeStatus selectedStatus =
      AuthoringRevision3VoiceTakeStatus.approved,
  String locId = _locId,
}) {
  var projectJson = revision3VoiceFixtureProjectWithExistingSlotJson(
    candidateCount: candidateCount,
    selectedStatus: selectedStatus,
  );
  if (locId != _locId) {
    final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
    final entities = (project['entities']! as Map).cast<String, Object?>();
    final localization = (entities[revision3VoiceFixtureLocalizationId]! as Map)
        .cast<String, Object?>();
    final payload = (localization['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();
    data['loc_id'] = locId;
    projectJson = jsonEncode(project);
  }
  return (projectJson: projectJson, head: _headFor(projectJson));
}

AuthoringRevision3VoiceTargetRequestV1 _request(
  ({String projectJson, AuthoringWorkingHead head}) basis, {
  String locId = _locId,
}) => AuthoringRevision3VoiceTargetRequestV1.forProject(
  expectedHead: basis.head,
  currentProjectJson: basis.projectJson,
  lineId: revision3VoiceFixtureLineId,
  slotId: revision3VoiceFixtureSlotId,
  locale: 'de',
  expectedLocId: locId,
);

Map<String, Object?> _target(
  String directory, {
  int crc32 = 42,
  String? member,
}) => <String, Object?>{
  'archive': _archive,
  'member': member ?? '$directory/$_locId.ogg',
  'operation': 'replace',
  'archive_seal': <String, Object?>{
    'byte_len': _archiveBytes,
    'sha256': _archiveSha,
  },
  'member_proof': <String, Object?>{
    'state': 'present',
    'uncompressed_size': 8192,
    'crc32': crc32,
  },
};

Map<String, Object?> _response({
  required ({String projectJson, AuthoringWorkingHead head}) basis,
  required AuthoringRevision3VoiceTargetRequestV1 request,
  required String resolution,
  String? resolvedMember,
}) {
  final targets = switch (resolution) {
    'unresolved' => <Map<String, Object?>>[],
    'resolved' => <Map<String, Object?>>[
      _target('Voices/Hero', member: resolvedMember),
    ],
    'ambiguous' => <Map<String, Object?>>[
      _target('Voices/A'),
      _target('Voices/B', crc32: 43),
    ],
    _ => throw ArgumentError.value(resolution),
  };
  final targetResolution = switch (resolution) {
    'unresolved' => <String, Object?>{'state': 'unresolved'},
    'resolved' => <String, Object?>{
      'state': 'resolved',
      'target': targets.single,
    },
    'ambiguous' => <String, Object?>{
      'state': 'ambiguous',
      'candidates': <Object?>[...targets],
    },
    _ => throw StateError('unreachable'),
  };
  final project = (jsonDecode(basis.projectJson) as Map)
      .cast<String, Object?>();
  project['revision'] = request.expectedRevision + 1;
  final entities = (project['entities']! as Map).cast<String, Object?>();
  final slot = (entities[revision3VoiceFixtureSlotId]! as Map)
      .cast<String, Object?>();
  slot['revision'] = (slot['revision']! as int) + 1;
  final payload = (slot['payload']! as Map).cast<String, Object?>();
  final data = (payload['data']! as Map).cast<String, Object?>();
  data['target_resolution'] = targetResolution;
  payload['data'] = data;
  slot['payload'] = payload;
  entities[revision3VoiceFixtureSlotId] = slot;
  project['entities'] = entities;
  final candidateProjectJson = jsonEncode(project);
  return <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': basis.head.canonicalJson,
    'head_json': _headFor(candidateProjectJson).canonicalJson,
    'project_json': candidateProjectJson,
    'revision': request.expectedRevision + 1,
    'line_id': revision3VoiceFixtureLineId,
    'localization_id': revision3VoiceFixtureLocalizationId,
    'slot_id': revision3VoiceFixtureSlotId,
    'locale': 'de',
    'loc_id': request.expectedLocId,
    'resolution': resolution,
    'match_count': targets.length,
    'target_resolution': targetResolution,
    'archive_observation': targets.isEmpty
        ? null
        : <String, Object?>{
            'archive': _archive,
            'archive_seal': <String, Object?>{
              'byte_len': _archiveBytes,
              'sha256': _archiveSha,
            },
          },
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_supported',
  };
}

void main() {
  test('required command handshake includes native R3 Voice target', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_prepare_revision3_voice_target_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('request is canonical and binds exact project generation and LocID', () {
    final basis = _voiceBasis();
    final request = _request(basis);
    final reopened = AuthoringRevision3VoiceTargetRequestV1.fromCanonicalJson(
      request.canonicalJson,
    );
    expect(reopened.expectedHead.canonicalJson, basis.head.canonicalJson);
    expect(reopened.expectedProjectId, revision3VoiceFixtureProjectId);
    expect(reopened.expectedRevision, 8);
    expect(reopened.expectedLocId, _locId);

    final raw = (jsonDecode(request.canonicalJson) as Map)
        .cast<String, Object?>();
    for (final malformed in <String>[
      ' ${request.canonicalJson}',
      jsonEncode(<String, Object?>{...raw, 'expected_loc_id': '../bad'}),
      jsonEncode(<String, Object?>{...raw, 'locale': 'de-de'}),
      jsonEncode(<String, Object?>{...raw}..remove('expected_target')),
    ]) {
      expect(
        () =>
            AuthoringRevision3VoiceTargetRequestV1.fromCanonicalJson(malformed),
        throwsFormatException,
      );
    }
  });

  test('target request enforces exact portable LocID stem boundaries', () {
    final basis = _voiceBasis();
    AuthoringRevision3VoiceTargetRequestV1 request(String locId) =>
        AuthoringRevision3VoiceTargetRequestV1.forProject(
          expectedHead: basis.head,
          currentProjectJson: basis.projectJson,
          lineId: revision3VoiceFixtureLineId,
          slotId: revision3VoiceFixtureSlotId,
          locale: 'de',
          expectedLocId: locId,
        );

    expect(request('A' * 1020).expectedLocId, hasLength(1020));
    for (final invalid in <String>[
      'A' * 1021,
      'CON',
      r'CLOCK$',
      'COM1.txt',
      'trailing.',
      'trailing ',
      'bad:name',
      'bad/name',
      r'bad\name',
      'nön-ascii',
    ]) {
      expect(() => request(invalid), throwsFormatException, reason: invalid);
    }
  });

  test('target response accepts a full slot and a reviewed selected take', () {
    for (final basis in <({String projectJson, AuthoringWorkingHead head})>[
      _voiceBasis(candidateCount: 1024),
      _voiceBasis(selectedStatus: AuthoringRevision3VoiceTakeStatus.reviewed),
    ]) {
      final request = _request(basis);
      final prepared = AuthoringRevision3VoiceTargetPreparation.fromJson(
        _response(basis: basis, request: request, resolution: 'resolved'),
        currentProjectJson: basis.projectJson,
        request: request,
      );
      expect(
        prepared.resolution,
        AuthoringRevision3VoiceTargetResolutionState.resolved,
      );
    }
  });

  test(
    'strict parser retains unresolved, resolved, and ambiguous evidence',
    () {
      for (final expected in <String>['unresolved', 'resolved', 'ambiguous']) {
        final basis = _voiceBasis();
        final request = _request(basis);
        final prepared = AuthoringRevision3VoiceTargetPreparation.fromJson(
          _response(basis: basis, request: request, resolution: expected),
          currentProjectJson: basis.projectJson,
          request: request,
        );
        expect(prepared.resolution.name, expected);
        expect(prepared.matchCount, switch (expected) {
          'unresolved' => 0,
          'resolved' => 1,
          _ => 2,
        });
        expect(
          prepared.archiveObservation,
          expected == 'unresolved' ? isNull : isNotNull,
        );
        expect(
          prepared.resolvedTarget,
          expected == 'resolved' ? isNotNull : isNull,
        );
      }
    },
  );

  test(
    'wrapper sends intent only and accepts exact unpublished candidate',
    () async {
      final basis = _voiceBasis();
      final request = _request(basis);
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_voice_target_v1': _response(
            basis: basis,
            request: request,
            resolution: 'resolved',
          ),
        },
      );
      final prepared = await ModFfi(core)
          .authoringStorePrepareRevision3VoiceTargetV1(
            root: _storeRoot,
            gameRoot: _gameRoot,
            currentProjectJson: basis.projectJson,
            request: request,
          );
      expect(prepared.resolvedTarget!.archive, _archive);
      expect(core.calls.single.payload, <String, Object?>{
        'current_project_json': basis.projectJson,
        'game_root': _gameRoot,
        'root': _storeRoot,
        'voice_target_request_json': request.canonicalJson,
      });
    },
  );

  test('response cannot forge target evidence or widen candidate delta', () {
    final basis = _voiceBasis();
    final request = _request(basis);
    final forgedEvidence = _response(
      basis: basis,
      request: request,
      resolution: 'resolved',
    );
    ((forgedEvidence['target_resolution']! as Map)['target']!
            as Map)['operation'] =
        'add';
    final widenedDelta = _response(
      basis: basis,
      request: request,
      resolution: 'resolved',
    );
    final widenedProject =
        (jsonDecode(widenedDelta['project_json']! as String) as Map)
            .cast<String, Object?>();
    widenedProject['authoring_locales'] = <Object?>['de', 'en'];
    widenedDelta['project_json'] = jsonEncode(widenedProject);
    widenedDelta['head_json'] = _headFor(
      widenedDelta['project_json']! as String,
    ).canonicalJson;

    for (final response in <Map<String, Object?>>[
      forgedEvidence,
      widenedDelta,
    ]) {
      expect(
        () => AuthoringRevision3VoiceTargetPreparation.fromJson(
          response,
          currentProjectJson: basis.projectJson,
          request: request,
        ),
        throwsFormatException,
      );
    }
  });

  test('response uses native ASCII-only LocID basename matching', () {
    final basis = _voiceBasis(locId: 'K');
    final request = _request(basis, locId: 'K');
    expect(
      () => AuthoringRevision3VoiceTargetPreparation.fromJson(
        _response(
          basis: basis,
          request: request,
          resolution: 'resolved',
          resolvedMember: 'Voices/Hero/K.ogg',
        ),
        currentProjectJson: basis.projectJson,
        request: request,
      ),
      throwsFormatException,
    );
  });
}
