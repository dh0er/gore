import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_voice_fixture.dart';
import '../support/revision3_voice_selection_fixture.dart';

const _root = r'C:\Projects\VoiceSelection.goreproj';

void main() {
  test('handshake includes sorted Voice take selection command', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_prepare_revision3_voice_take_selection_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('request has exact order, nullable CAS selection, and no game root', () {
    final fixture = Revision3VoiceSelectionFixture();
    final request = fixture.request();
    final wire = jsonDecode(request.canonicalJson) as Map<String, Object?>;

    expect(wire.keys, <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'line_id',
      'slot_id',
      'expected_slot_revision',
      'locale',
      'expected_loc_id',
      'expected_selected_take_id',
      'selected_take_id',
    ]);
    expect(wire, isNot(contains('game_root')));
    expect(wire, isNot(contains('asset')));
    expect(wire, isNot(contains('source')));
    expect(request.expectedSelectedTakeId, revision3VoiceFixtureTakeId);
    expect(request.selectedTakeId, revision3VoiceSelectionAlternateTakeId);

    final clear = Revision3VoiceSelectionFixture(clear: true).request();
    expect(
      (jsonDecode(clear.canonicalJson)
          as Map<String, Object?>)['selected_take_id'],
      isNull,
    );
  });

  test('request rejects no-op, noncandidate, and nonapproved selections', () {
    final fixture = Revision3VoiceSelectionFixture();
    expect(
      () => AuthoringRevision3VoiceTakeSelectionRequestV1.forProject(
        expectedHead: fixture.head,
        currentProjectJson: fixture.projectJson,
        lineId: revision3VoiceFixtureLineId,
        slotId: revision3VoiceFixtureSlotId,
        expectedSlotRevision: fixture.slotRevision,
        locale: 'de',
        expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
        expectedSelectedTakeId: revision3VoiceFixtureTakeId,
        selectedTakeId: revision3VoiceFixtureTakeId,
      ),
      throwsFormatException,
    );
    expect(
      () => AuthoringRevision3VoiceTakeSelectionRequestV1.forProject(
        expectedHead: fixture.head,
        currentProjectJson: fixture.projectJson,
        lineId: revision3VoiceFixtureLineId,
        slotId: revision3VoiceFixtureSlotId,
        expectedSlotRevision: fixture.slotRevision,
        locale: 'de',
        expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
        expectedSelectedTakeId: revision3VoiceFixtureTakeId,
        selectedTakeId: '99999999999999999999999999999999',
      ),
      throwsFormatException,
    );

    final reviewedProject = revision3VoiceFixtureProjectWithExistingSlotJson(
      candidateCount: 2,
      selectedStatus: AuthoringRevision3VoiceTakeStatus.reviewed,
    );
    expect(
      () => AuthoringRevision3VoiceTakeSelectionRequestV1.forProject(
        expectedHead: fixture.head,
        currentProjectJson: reviewedProject,
        lineId: revision3VoiceFixtureLineId,
        slotId: revision3VoiceFixtureSlotId,
        expectedSlotRevision: fixture.slotRevision,
        locale: 'de',
        expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
        expectedSelectedTakeId: revision3VoiceFixtureTakeId,
        selectedTakeId: revision3VoiceSelectionAlternateTakeId,
      ),
      throwsFormatException,
    );
  });

  test(
    'FFI uses exact project-only payload and accepts manifest head',
    () async {
      final fixture = Revision3VoiceSelectionFixture();
      final projectBytes = utf8Bytes(fixture.projectJson);
      expect(fixture.head.snapshotByteLength, isNot(projectBytes.length));
      expect(
        fixture.head.snapshotSha256,
        isNot(crypto.sha256.convert(projectBytes).toString()),
      );
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_store_prepare_revision3_voice_take_selection_v1': fixture
              .response(),
        },
      );

      final prepared = await ModFfi(core)
          .authoringStorePrepareRevision3VoiceTakeSelectionV1(
            root: _root,
            currentProjectJson: fixture.projectJson,
            request: fixture.request(),
          );

      expect(prepared.projectId, revision3VoiceFixtureProjectId);
      expect(prepared.revision, fixture.projectRevision + 1);
      expect(prepared.slotRevision, fixture.slotRevision + 1);
      expect(prepared.previousSelectedTakeId, revision3VoiceFixtureTakeId);
      expect(prepared.selectedTakeId, revision3VoiceSelectionAlternateTakeId);
      expect(
        prepared.buildStatus,
        AuthoringRevision3VoiceTakeSelectionBuildStatus.blocked,
      );
      expect(
        prepared.runtimeStatus,
        AuthoringRevision3VoiceTakeSelectionRuntimeStatus.runtimeUnqualified,
      );
      expect(
        prepared.publicationStatus,
        AuthoringRevision3VoiceTakeSelectionPublicationStatus.notSupported,
      );
      expect(core.calls.single.payload.keys, <String>[
        'current_project_json',
        'root',
        'voice_take_selection_request_json',
      ]);
      expect(core.calls.single.payload, isNot(contains('game_root')));
    },
  );

  test(
    'response rejects every delta beyond project and slot selection',
    () async {
      final fixture = Revision3VoiceSelectionFixture();
      final response = fixture.response();
      final candidate =
          jsonDecode(response['project_json']! as String)
              as Map<String, Object?>;
      (candidate['meta']! as Map<String, Object?>)['name'] = 'Smuggled';
      response['project_json'] = jsonEncode(candidate);

      await expectLater(
        ModFfi(
          FakeGoreCoreFfiService(
            responses: {
              'authoring_store_prepare_revision3_voice_take_selection_v1':
                  response,
            },
          ),
        ).authoringStorePrepareRevision3VoiceTakeSelectionV1(
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
    },
  );

  test(
    'response rejects status escalation and selection disagreement',
    () async {
      final fixture = Revision3VoiceSelectionFixture();
      for (final mutation in <void Function(Map<String, Object?>)>[
        (response) => response['build_status'] = 'ready',
        (response) => response['runtime_status'] = 'runtime_ready',
        (response) => response['publication_status'] = 'published',
        (response) => response['selected_take_id'] = null,
      ]) {
        final response = fixture.response();
        mutation(response);
        await expectLater(
          ModFfi(
            FakeGoreCoreFfiService(
              responses: {
                'authoring_store_prepare_revision3_voice_take_selection_v1':
                    response,
              },
            ),
          ).authoringStorePrepareRevision3VoiceTakeSelectionV1(
            root: _root,
            currentProjectJson: fixture.projectJson,
            request: fixture.request(),
          ),
          throwsA(isA<ModFfiException>()),
        );
      }
    },
  );
}

Uint8List utf8Bytes(String value) => Uint8List.fromList(utf8.encode(value));
