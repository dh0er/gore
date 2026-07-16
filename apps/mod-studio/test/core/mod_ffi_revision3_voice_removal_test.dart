import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_voice_fixture.dart';
import '../support/revision3_voice_removal_fixture.dart';

const _root = r'C:\Projects\VoiceRemoval.goreproj';

void main() {
  test('handshake includes sorted Voice take removal command', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_prepare_revision3_voice_take_removal_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('request is strict, canonical, exact-current, and project-only', () {
    final fixture = Revision3VoiceRemovalFixture();
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
      'expected_selected_take_id',
    ]);
    expect(wire, isNot(contains('game_root')));
    expect(wire, isNot(contains('source')));
    expect(wire, isNot(contains('output')));
    expect(request.takeId, revision3VoiceFixtureTakeId);
    expect(request.expectedSelectedTakeId, revision3VoiceFixtureTakeId);

    final reordered = <String, Object?>{
      'expected_project_id': wire['expected_project_id'],
      'expected_head': wire['expected_head'],
      for (final entry in wire.entries)
        if (entry.key != 'expected_project_id' && entry.key != 'expected_head')
          entry.key: entry.value,
    };
    expect(
      () => AuthoringRevision3VoiceTakeRemovalRequestV1.fromCanonicalJson(
        jsonEncode(reordered),
        currentProjectJson: fixture.projectJson,
      ),
      throwsFormatException,
    );
  });

  test(
    'last candidate removes entity but preserves immutable asset store',
    () async {
      final fixture = Revision3VoiceRemovalFixture(candidateCount: 1);
      final prepared = await _prepare(fixture);
      final before = jsonDecode(fixture.projectJson) as Map<String, Object?>;
      final after = jsonDecode(prepared.projectJson) as Map<String, Object?>;

      expect(prepared.selectionCleared, isTrue);
      expect(prepared.takeEntityRemoved, isTrue);
      expect(prepared.remainingCandidateCount, 0);
      expect(
        (after['entities']! as Map),
        isNot(contains(revision3VoiceFixtureTakeId)),
      );
      expect(after['asset_store'], before['asset_store']);
    },
  );

  test('shared take is detached locally and retained byte-for-byte', () async {
    final fixture = Revision3VoiceRemovalFixture(shared: true);
    final prepared = await _prepare(fixture);
    final before = jsonDecode(fixture.projectJson) as Map<String, Object?>;
    final after = jsonDecode(prepared.projectJson) as Map<String, Object?>;
    final beforeEntities = (before['entities']! as Map).cast<String, Object?>();
    final afterEntities = (after['entities']! as Map).cast<String, Object?>();

    expect(prepared.takeEntityRemoved, isFalse);
    expect(
      afterEntities[fixture.takeId],
      equals(beforeEntities[fixture.takeId]),
    );
    expect(afterEntities, contains(revision3VoiceRemovalSharedSlotId));
  });

  test('nonselected removal preserves selection and survivor order', () async {
    const alternate = '00000000000000000000000000001001';
    final fixture = Revision3VoiceRemovalFixture(takeId: alternate);
    final prepared = await _prepare(fixture);
    final project = jsonDecode(prepared.projectJson) as Map<String, Object?>;
    final entities = (project['entities']! as Map).cast<String, Object?>();
    final slot = (entities[revision3VoiceFixtureSlotId]! as Map)
        .cast<String, Object?>();
    final payload = (slot['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();

    expect(prepared.selectionCleared, isFalse);
    expect((data['selected']! as Map)['id'], revision3VoiceFixtureTakeId);
    expect(
      (data['candidates']! as List).map((item) => (item as Map)['id']),
      <String>[revision3VoiceFixtureTakeId],
    );
  });

  test(
    'transport uses exact payload and rejects receipt or delta smuggling',
    () async {
      final fixture = Revision3VoiceRemovalFixture();
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_store_prepare_revision3_voice_take_removal_v1': fixture
              .response(),
        },
      );
      await ModFfi(core).authoringStorePrepareRevision3VoiceTakeRemovalV1(
        root: _root,
        currentProjectJson: fixture.projectJson,
        request: fixture.request(),
      );
      expect(core.calls.single.payload.keys, <String>[
        'current_project_json',
        'root',
        'voice_take_removal_request_json',
      ]);

      for (final mutate in <void Function(Map<String, Object?>)>[
        (response) => response['take_entity_removed'] = false,
        (response) => response['selection_cleared'] = false,
        (response) => response['build_status'] = 'ready',
        (response) {
          final project =
              jsonDecode(response['project_json']! as String)
                  as Map<String, Object?>;
          (project['meta']! as Map)['name'] = 'Smuggled';
          response['project_json'] = jsonEncode(project);
        },
      ]) {
        final response = fixture.response();
        mutate(response);
        await expectLater(
          ModFfi(
            FakeGoreCoreFfiService(
              responses: {
                'authoring_store_prepare_revision3_voice_take_removal_v1':
                    response,
              },
            ),
          ).authoringStorePrepareRevision3VoiceTakeRemovalV1(
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
      }
    },
  );
}

Future<AuthoringRevision3VoiceTakeRemovalPreparation> _prepare(
  Revision3VoiceRemovalFixture fixture,
) =>
    ModFfi(
      FakeGoreCoreFfiService(
        responses: {
          'authoring_store_prepare_revision3_voice_take_removal_v1': fixture
              .response(),
        },
      ),
    ).authoringStorePrepareRevision3VoiceTakeRemovalV1(
      root: _root,
      currentProjectJson: fixture.projectJson,
      request: fixture.request(),
    );
