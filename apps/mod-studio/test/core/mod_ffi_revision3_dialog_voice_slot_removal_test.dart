import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';

import '../support/revision3_dialog_voice_slot_removal_fixture.dart';

const _root = r'C:\Projects\DialogVoiceSlotRemoval.goreproj';

void main() {
  test('handshake requires sorted dialog Voice slot removal command', () {
    expect(
      requiredStudioCoreCommands,
      contains(
        'authoring_store_prepare_revision3_dialog_voice_slot_removal_v1',
      ),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('request is canonical, exact-current, ordered, and project-only', () {
    final fixture = Revision3DialogVoiceSlotRemovalFixture();
    final request = fixture.request();
    final wire = jsonDecode(request.canonicalJson) as Map<String, Object?>;

    expect(wire.keys, <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'line_id',
      'expected_line_revision',
      'localization_id',
      'expected_loc_id',
      'locale',
      'slot_id',
      'expected_slot_revision',
    ]);
    expect(wire, isNot(contains('game_root')));
    expect(wire, isNot(contains('source')));
    expect(wire, isNot(contains('output')));

    final reordered = <String, Object?>{
      'expected_project_id': wire['expected_project_id'],
      'expected_head': wire['expected_head'],
      for (final entry in wire.entries)
        if (entry.key != 'expected_project_id' && entry.key != 'expected_head')
          entry.key: entry.value,
    };
    expect(
      () => AuthoringRevision3DialogVoiceSlotRemovalRequestV1.fromCanonicalJson(
        jsonEncode(reordered),
        currentProjectJson: fixture.projectJson,
      ),
      throwsFormatException,
    );
  });

  test('preparation closes exact line-slot removal delta', () async {
    final fixture = Revision3DialogVoiceSlotRemovalFixture();
    final prepared = await _prepare(fixture);
    final before = jsonDecode(fixture.projectJson) as Map<String, Object?>;
    final after = jsonDecode(prepared.projectJson) as Map<String, Object?>;
    final beforeEntities = (before['entities']! as Map).cast<String, Object?>();
    final afterEntities = (after['entities']! as Map).cast<String, Object?>();

    expect(prepared.lineRevision, fixture.lineRevision + 1);
    expect(prepared.removedSlotRevision, fixture.slotRevision);
    expect(
      prepared.removedTargetResolution,
      Revision3ContentVoiceTargetResolution.unresolved,
    );
    expect(
      afterEntities,
      isNot(contains(revision3DialogVoiceSlotRemovalSlotId)),
    );
    expect(after['asset_store'], before['asset_store']);
    expect(afterEntities.length, beforeEntities.length - 1);
  });

  test(
    'preparation rejects a slot outside the managed generated origin',
    () async {
      final fixture = Revision3DialogVoiceSlotRemovalFixture();
      final project = fixture.project;
      final entities = (project['entities']! as Map).cast<String, Object?>();
      final slot = (entities[revision3DialogVoiceSlotRemovalSlotId]! as Map)
          .cast<String, Object?>();
      slot['origin'] = <String, Object?>{
        'type': 'new',
        'authored_runtime_id': 'not-managed-generated',
      };
      entities[revision3DialogVoiceSlotRemovalSlotId] = slot;
      project['entities'] = entities;

      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_store_prepare_revision3_dialog_voice_slot_removal_v1':
              fixture.response(),
        },
      );
      await expectLater(
        ModFfi(core).authoringStorePrepareRevision3DialogVoiceSlotRemovalV1(
          root: _root,
          currentProjectJson: jsonEncode(project),
          request: fixture.request(),
        ),
        throwsFormatException,
      );
      expect(
        core.calls,
        isEmpty,
        reason: 'managed preflight must reject first',
      );
    },
  );

  test(
    'transport uses exact payload and rejects authority or delta smuggling',
    () async {
      final fixture = Revision3DialogVoiceSlotRemovalFixture();
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_store_prepare_revision3_dialog_voice_slot_removal_v1':
              fixture.response(),
        },
      );
      await ModFfi(core).authoringStorePrepareRevision3DialogVoiceSlotRemovalV1(
        root: _root,
        currentProjectJson: fixture.projectJson,
        request: fixture.request(),
      );
      expect(core.calls.single.payload.keys, <String>[
        'current_project_json',
        'root',
        'dialog_voice_slot_removal_request_json',
      ]);

      for (final mutate in <void Function(Map<String, Object?>)>[
        (response) => response['line_revision'] = fixture.lineRevision,
        (response) => response['removed_target_resolution'] = 'resolved',
        (response) => response['target_authority'] = 'granted',
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
                'authoring_store_prepare_revision3_dialog_voice_slot_removal_v1':
                    response,
              },
            ),
          ).authoringStorePrepareRevision3DialogVoiceSlotRemovalV1(
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

Future<AuthoringRevision3DialogVoiceSlotRemovalPreparation> _prepare(
  Revision3DialogVoiceSlotRemovalFixture fixture,
) =>
    ModFfi(
      FakeGoreCoreFfiService(
        responses: {
          'authoring_store_prepare_revision3_dialog_voice_slot_removal_v1':
              fixture.response(),
        },
      ),
    ).authoringStorePrepareRevision3DialogVoiceSlotRemovalV1(
      root: _root,
      currentProjectJson: fixture.projectJson,
      request: fixture.request(),
    );
