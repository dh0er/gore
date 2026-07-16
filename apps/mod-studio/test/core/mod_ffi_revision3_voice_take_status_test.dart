import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_voice_fixture.dart';
import '../support/revision3_voice_selection_fixture.dart';

const _command = 'authoring_store_prepare_revision3_voice_take_status_v1';
const _root = r'C:\Projects\VoiceTakeStatus.goreproj';
const _locId = 'GRD_263_ASGHAN_OPEN_INFO_06_02';

void main() {
  test('handshake includes sorted Voice take status command', () {
    expect(requiredStudioCoreCommands, contains(_command));
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test(
    'request mirrors the native exact field order and authority boundary',
    () {
      final fixture = _VoiceTakeStatusFixture();
      final request = fixture.request();
      final wire = jsonDecode(request.canonicalJson) as Map<String, Object?>;

      expect(wire.keys, <String>[
        'expected_head',
        'expected_project_id',
        'expected_revision',
        'expected_target',
        'line_id',
        'localization_id',
        'expected_loc_id',
        'locale',
        'slot_id',
        'expected_slot_revision',
        'take_id',
        'expected_take_revision',
        'expected_status',
        'desired_status',
      ]);
      expect(wire['expected_status'], 'recorded');
      expect(wire['desired_status'], 'reviewed');
      expect(wire, isNot(contains('game_root')));
      expect(wire, isNot(contains('source')));
      expect(wire, isNot(contains('audio')));
      expect(wire, isNot(contains('asset')));
    },
  );

  test('request rejects no-op and selected-take demotion locally', () {
    final fixture = _VoiceTakeStatusFixture();
    expect(
      () => fixture.request(
        desiredStatus: AuthoringRevision3VoiceTakeStatus.recorded,
      ),
      throwsFormatException,
    );

    final approvedProject = revision3VoiceFixtureProjectWithExistingSlotJson(
      candidateCount: 2,
    );
    expect(
      () => _requestForProject(
        projectJson: approvedProject,
        head: fixture.head,
        takeId: revision3VoiceFixtureTakeId,
        expectedStatus: AuthoringRevision3VoiceTakeStatus.approved,
        desiredStatus: AuthoringRevision3VoiceTakeStatus.reviewed,
      ),
      throwsFormatException,
    );
  });

  test('request rejects stale graph and take CAS values', () {
    final fixture = _VoiceTakeStatusFixture();
    for (final build in <AuthoringRevision3VoiceTakeStatusRequestV1 Function()>[
      () => _requestForProject(
        projectJson: fixture.projectJson,
        head: fixture.head,
        localizationId: '99999999999999999999999999999991',
      ),
      () => _requestForProject(
        projectJson: fixture.projectJson,
        head: fixture.head,
        slotId: '99999999999999999999999999999992',
      ),
      () => _requestForProject(
        projectJson: fixture.projectJson,
        head: fixture.head,
        takeId: '99999999999999999999999999999993',
      ),
      () => _requestForProject(
        projectJson: fixture.projectJson,
        head: fixture.head,
        expectedSlotRevision: fixture.slotRevision + 1,
      ),
      () => _requestForProject(
        projectJson: fixture.projectJson,
        head: fixture.head,
        expectedTakeRevision: fixture.takeRevision + 1,
      ),
      () => _requestForProject(
        projectJson: fixture.projectJson,
        head: fixture.head,
        expectedStatus: AuthoringRevision3VoiceTakeStatus.draft,
      ),
    ]) {
      expect(build, throwsFormatException);
    }
  });

  test('request parser rejects reordered and noncanonical JSON', () {
    final fixture = _VoiceTakeStatusFixture();
    final request = fixture.request();
    final wire = (jsonDecode(request.canonicalJson) as Map)
        .cast<String, Object?>();
    final reordered = <String, Object?>{
      'desired_status': wire['desired_status'],
      for (final entry in wire.entries)
        if (entry.key != 'desired_status') entry.key: entry.value,
    };

    expect(
      () => AuthoringRevision3VoiceTakeStatusRequestV1.fromCanonicalJson(
        jsonEncode(reordered),
        currentProjectJson: fixture.projectJson,
      ),
      throwsFormatException,
    );
    expect(
      () => AuthoringRevision3VoiceTakeStatusRequestV1.fromCanonicalJson(
        request.canonicalJson.replaceFirst('{', '{ '),
        currentProjectJson: fixture.projectJson,
      ),
      throwsFormatException,
    );
  });

  test('unchanged VoiceSlot accepts the signed wire revision maximum', () {
    final fixture = _VoiceTakeStatusFixture();
    final project = (jsonDecode(fixture.projectJson) as Map)
        .cast<String, Object?>();
    final entities = (project['entities']! as Map).cast<String, Object?>();
    final slot = (entities[revision3VoiceFixtureSlotId]! as Map)
        .cast<String, Object?>();
    slot['revision'] = 0x7fffffffffffffff;
    final projectJson = jsonEncode(project);

    final request = _requestForProject(
      projectJson: projectJson,
      head: fixture.head,
      expectedSlotRevision: 0x7fffffffffffffff,
      expectedTakeRevision: fixture.takeRevision,
    );

    expect(request.expectedSlotRevision, 0x7fffffffffffffff);
  });

  test(
    'FFI sends only exact project/status payload and parses receipt',
    () async {
      final fixture = _VoiceTakeStatusFixture();
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{_command: fixture.response()},
      );

      final prepared = await ModFfi(core)
          .authoringStorePrepareRevision3VoiceTakeStatusV1(
            root: _root,
            currentProjectJson: fixture.projectJson,
            request: fixture.request(),
          );

      expect(prepared.projectId, revision3VoiceFixtureProjectId);
      expect(prepared.revision, fixture.projectRevision + 1);
      expect(prepared.lineId, revision3VoiceFixtureLineId);
      expect(prepared.localizationId, revision3VoiceFixtureLocalizationId);
      expect(prepared.slotId, revision3VoiceFixtureSlotId);
      expect(prepared.slotRevision, fixture.slotRevision);
      expect(prepared.takeId, revision3VoiceSelectionAlternateTakeId);
      expect(prepared.takeRevision, fixture.takeRevision + 1);
      expect(
        prepared.previousStatus,
        AuthoringRevision3VoiceTakeStatus.recorded,
      );
      expect(prepared.status, AuthoringRevision3VoiceTakeStatus.reviewed);
      expect(
        prepared.buildStatus,
        AuthoringRevision3VoiceTakeStatusBuildStatus.blocked,
      );
      expect(
        prepared.runtimeStatus,
        AuthoringRevision3VoiceTakeStatusRuntimeStatus.runtimeUnqualified,
      );
      expect(
        prepared.publicationStatus,
        AuthoringRevision3VoiceTakeStatusPublicationStatus.notSupported,
      );
      expect(core.calls.single.payload.keys, <String>[
        'current_project_json',
        'root',
        'voice_take_status_request_json',
      ]);
      expect(core.calls.single.payload, isNot(contains('game_root')));
    },
  );

  test(
    'response rejects every delta beyond project/take revision and status',
    () async {
      final fixture = _VoiceTakeStatusFixture();
      for (final mutate in <void Function(Map<String, Object?>)>[
        (response) {
          final candidate =
              (jsonDecode(response['project_json']! as String) as Map)
                  .cast<String, Object?>();
          (candidate['meta']! as Map<String, Object?>)['name'] = 'Smuggled';
          response['project_json'] = jsonEncode(candidate);
        },
        (response) {
          final candidate =
              (jsonDecode(response['project_json']! as String) as Map)
                  .cast<String, Object?>();
          final entities = (candidate['entities']! as Map)
              .cast<String, Object?>();
          final slot = (entities[revision3VoiceFixtureSlotId]! as Map)
              .cast<String, Object?>();
          slot['revision'] = fixture.slotRevision + 1;
          response['project_json'] = jsonEncode(candidate);
        },
      ]) {
        final response = fixture.response();
        mutate(response);
        await _expectMalformed(fixture, response);
      }
    },
  );

  test('response rejects authority, identity, and status escalation', () async {
    final fixture = _VoiceTakeStatusFixture();
    for (final mutate in <void Function(Map<String, Object?>)>[
      (response) => response['build_status'] = 'ready',
      (response) => response['runtime_status'] = 'runtime_ready',
      (response) => response['publication_status'] = 'published',
      (response) => response['slot_revision'] = fixture.slotRevision + 1,
      (response) => response['take_revision'] = fixture.takeRevision + 2,
      (response) => response['previous_status'] = 'draft',
      (response) => response['status'] = 'approved',
      (response) => response['localization_id'] = revision3VoiceFixtureLineId,
      (response) => response['extra'] = true,
    ]) {
      final response = fixture.response();
      mutate(response);
      await _expectMalformed(fixture, response);
    }
  });
}

AuthoringRevision3VoiceTakeStatusRequestV1 _requestForProject({
  required String projectJson,
  required AuthoringWorkingHead head,
  String lineId = revision3VoiceFixtureLineId,
  String localizationId = revision3VoiceFixtureLocalizationId,
  String expectedLocId = _locId,
  String locale = 'de',
  String slotId = revision3VoiceFixtureSlotId,
  int expectedSlotRevision = 0,
  String takeId = revision3VoiceSelectionAlternateTakeId,
  int expectedTakeRevision = 0,
  AuthoringRevision3VoiceTakeStatus expectedStatus =
      AuthoringRevision3VoiceTakeStatus.recorded,
  AuthoringRevision3VoiceTakeStatus desiredStatus =
      AuthoringRevision3VoiceTakeStatus.reviewed,
}) => AuthoringRevision3VoiceTakeStatusRequestV1.forProject(
  expectedHead: head,
  currentProjectJson: projectJson,
  lineId: lineId,
  localizationId: localizationId,
  expectedLocId: expectedLocId,
  locale: locale,
  slotId: slotId,
  expectedSlotRevision: expectedSlotRevision,
  takeId: takeId,
  expectedTakeRevision: expectedTakeRevision,
  expectedStatus: expectedStatus,
  desiredStatus: desiredStatus,
);

Future<void> _expectMalformed(
  _VoiceTakeStatusFixture fixture,
  Map<String, Object?> response,
) => expectLater(
  ModFfi(
    FakeGoreCoreFfiService(
      responses: <String, Map<String, Object?>>{_command: response},
    ),
  ).authoringStorePrepareRevision3VoiceTakeStatusV1(
    root: _root,
    currentProjectJson: fixture.projectJson,
    request: fixture.request(),
  ),
  throwsA(
    isA<ModFfiException>().having(
      (error) => error.code,
      'code',
      ModFfiException.malformedNativeResponseCode,
    ),
  ),
);

final class _VoiceTakeStatusFixture {
  _VoiceTakeStatusFixture()
    : projectJson = revision3VoiceFixtureProjectWithExistingSlotJson(
        candidateCount: 2,
        selectedStatus: AuthoringRevision3VoiceTakeStatus.recorded,
      ),
      head = revision3VoiceSelectionManifestHead('b', byteLength: 733),
      candidateHead = revision3VoiceSelectionManifestHead('c', byteLength: 911);

  final String projectJson;
  final AuthoringWorkingHead head;
  final AuthoringWorkingHead candidateHead;

  int get projectRevision =>
      (jsonDecode(projectJson) as Map<String, Object?>)['revision']! as int;

  int get slotRevision => _entityRevision(revision3VoiceFixtureSlotId);

  int get takeRevision =>
      _entityRevision(revision3VoiceSelectionAlternateTakeId);

  int _entityRevision(String id) {
    final project = jsonDecode(projectJson) as Map<String, Object?>;
    final entities = (project['entities']! as Map).cast<String, Object?>();
    final entity = (entities[id]! as Map).cast<String, Object?>();
    return entity['revision']! as int;
  }

  AuthoringRevision3VoiceTakeStatusRequestV1 request({
    AuthoringRevision3VoiceTakeStatus desiredStatus =
        AuthoringRevision3VoiceTakeStatus.reviewed,
  }) => _requestForProject(
    projectJson: projectJson,
    head: head,
    expectedSlotRevision: slotRevision,
    expectedTakeRevision: takeRevision,
    desiredStatus: desiredStatus,
  );

  String candidateProjectJson() {
    final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
    project['revision'] = projectRevision + 1;
    final entities = (project['entities']! as Map).cast<String, Object?>();
    final take = (entities[revision3VoiceSelectionAlternateTakeId]! as Map)
        .cast<String, Object?>();
    take['revision'] = takeRevision + 1;
    final payload = (take['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();
    data['status'] = 'reviewed';
    payload['data'] = data;
    take['payload'] = payload;
    entities[revision3VoiceSelectionAlternateTakeId] = take;
    project['entities'] = entities;
    return jsonEncode(project);
  }

  Map<String, Object?> response() => <String, Object?>{
    'ok': true,
    'outcome': 'prepared_unpublished',
    'basis_head_json': head.canonicalJson,
    'head_json': candidateHead.canonicalJson,
    'project_json': candidateProjectJson(),
    'project_id': revision3VoiceFixtureProjectId,
    'revision': projectRevision + 1,
    'line_id': revision3VoiceFixtureLineId,
    'localization_id': revision3VoiceFixtureLocalizationId,
    'slot_id': revision3VoiceFixtureSlotId,
    'slot_revision': slotRevision,
    'locale': 'de',
    'loc_id': _locId,
    'take_id': revision3VoiceSelectionAlternateTakeId,
    'take_revision': takeRevision + 1,
    'previous_status': 'recorded',
    'status': 'reviewed',
    'build_status': 'blocked',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_supported',
  };
}
