import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_content_index.dart';

import '../support/revision3_dialog_voice_slot_creation_fixture.dart';
import '../support/revision3_voice_fixture.dart';

const _root = r'C:\Projects\DialogVoiceSlotCreation.goreproj';

void main() {
  test('handshake requires sorted dialog Voice slot creation command', () {
    expect(
      requiredStudioCoreCommands,
      contains(
        'authoring_store_prepare_revision3_dialog_voice_slot_creation_v1',
      ),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('request is canonical, exact-current, ordered, and project-only', () {
    final fixture = Revision3DialogVoiceSlotCreationFixture();
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
    ]);
    expect(wire, isNot(contains('expected_localization_revision')));
    expect(wire, isNot(contains('slot_display_name')));
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
      () =>
          AuthoringRevision3DialogVoiceSlotCreationRequestV1.fromCanonicalJson(
            jsonEncode(reordered),
            currentProjectJson: fixture.projectJson,
          ),
      throwsFormatException,
    );

    final extra = Map<String, Object?>.from(wire)
      ..['slot_display_name'] = 'Caller-controlled';
    expect(
      () =>
          AuthoringRevision3DialogVoiceSlotCreationRequestV1.fromCanonicalJson(
            jsonEncode(extra),
            currentProjectJson: fixture.projectJson,
          ),
      throwsFormatException,
    );
  });

  test('preparation closes exact line-slot creation delta', () async {
    final fixture = Revision3DialogVoiceSlotCreationFixture();
    final prepared = await _prepare(fixture);
    final before = jsonDecode(fixture.projectJson) as Map<String, Object?>;
    final after = jsonDecode(prepared.projectJson) as Map<String, Object?>;
    final beforeEntities = (before['entities']! as Map).cast<String, Object?>();
    final afterEntities = (after['entities']! as Map).cast<String, Object?>();
    final slot = (afterEntities[revision3DialogVoiceSlotCreationSlotId]! as Map)
        .cast<String, Object?>();
    final payload = (slot['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();

    expect(prepared.lineRevision, fixture.lineRevision + 1);
    expect(prepared.localizationRevision, fixture.localizationRevision);
    expect(prepared.slotRevision, 0);
    expect(
      prepared.targetResolution,
      Revision3ContentVoiceTargetResolution.unresolved,
    );
    expect(slot['display_name'], 'Voice de');
    expect(slot['revision'], 0);
    expect(data['candidates'], isEmpty);
    expect(data, isNot(contains('selected')));
    expect(after['asset_store'], before['asset_store']);
    expect(after['authoring_locales'], before['authoring_locales']);
    expect(
      afterEntities[revision3VoiceFixtureLocalizationId],
      beforeEntities[revision3VoiceFixtureLocalizationId],
    );
    expect(afterEntities.length, beforeEntities.length + 1);
  });

  test('unchanged maximum signed localization revision remains valid', () async {
    const maxSignedRevision = 0x7fffffffffffffff;
    final fixture = Revision3DialogVoiceSlotCreationFixture();
    final request = fixture.request();
    final basis = fixture.project;
    final basisEntities = (basis['entities']! as Map).cast<String, Object?>();
    final basisLocalization =
        (basisEntities[revision3VoiceFixtureLocalizationId]! as Map)
            .cast<String, Object?>();
    basisLocalization['revision'] = maxSignedRevision;

    final response = fixture.response();
    final candidate =
        jsonDecode(response['project_json']! as String) as Map<String, Object?>;
    final candidateEntities = (candidate['entities']! as Map)
        .cast<String, Object?>();
    final candidateLocalization =
        (candidateEntities[revision3VoiceFixtureLocalizationId]! as Map)
            .cast<String, Object?>();
    candidateLocalization['revision'] = maxSignedRevision;
    response['project_json'] = jsonEncode(candidate);
    response['localization_revision'] = maxSignedRevision;

    final prepared =
        await ModFfi(
          FakeGoreCoreFfiService(
            responses: {
              'authoring_store_prepare_revision3_dialog_voice_slot_creation_v1':
                  response,
            },
          ),
        ).authoringStorePrepareRevision3DialogVoiceSlotCreationV1(
          root: _root,
          currentProjectJson: jsonEncode(basis),
          request: request,
        );

    expect(prepared.localizationRevision, maxSignedRevision);
  });

  test('preflight requires authorable locale, text, and absent slot', () async {
    final fixture = Revision3DialogVoiceSlotCreationFixture();
    final request = fixture.request();
    for (final mutate in <void Function(Map<String, Object?>)>[
      (project) => project['authoring_locales'] = <Object?>[],
      (project) {
        final entities = (project['entities']! as Map).cast<String, Object?>();
        final localization =
            (entities[revision3VoiceFixtureLocalizationId]! as Map)
                .cast<String, Object?>();
        final payload = (localization['payload']! as Map)
            .cast<String, Object?>();
        final data = (payload['data']! as Map).cast<String, Object?>();
        data['texts'] = <String, Object?>{};
      },
      (project) {
        final entities = (project['entities']! as Map).cast<String, Object?>();
        final line = (entities[revision3VoiceFixtureLineId]! as Map)
            .cast<String, Object?>();
        final payload = (line['payload']! as Map).cast<String, Object?>();
        final data = (payload['data']! as Map).cast<String, Object?>();
        data['voice_slots'] = <String, Object?>{
          'de': <String, Object?>{
            'project_id': revision3VoiceFixtureProjectId,
            'id': revision3DialogVoiceSlotCreationSlotId,
            'expected_kind': 'voice_slot',
          },
        };
      },
    ]) {
      final project = fixture.project;
      mutate(project);
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_store_prepare_revision3_dialog_voice_slot_creation_v1':
              fixture.response(),
        },
      );
      await expectLater(
        ModFfi(core).authoringStorePrepareRevision3DialogVoiceSlotCreationV1(
          root: _root,
          currentProjectJson: jsonEncode(project),
          request: request,
        ),
        throwsFormatException,
      );
      expect(core.calls, isEmpty, reason: 'closed preflight must reject first');
    }
  });

  test(
    'transport uses exact payload and rejects authority or delta smuggling',
    () async {
      final fixture = Revision3DialogVoiceSlotCreationFixture();
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_store_prepare_revision3_dialog_voice_slot_creation_v1':
              fixture.response(),
        },
      );
      await ModFfi(
        core,
      ).authoringStorePrepareRevision3DialogVoiceSlotCreationV1(
        root: _root,
        currentProjectJson: fixture.projectJson,
        request: fixture.request(),
      );
      expect(core.calls.single.payload.keys, <String>[
        'current_project_json',
        'root',
        'dialog_voice_slot_creation_request_json',
      ]);

      for (final mutate in <void Function(Map<String, Object?>)>[
        (response) => response['line_revision'] = fixture.lineRevision,
        (response) => response['localization_revision'] =
            fixture.localizationRevision + 1,
        (response) => response['slot_revision'] = 1,
        (response) => response['target_resolution'] = 'resolved',
        (response) => response['target_authority'] = 'granted',
        (response) {
          final project =
              jsonDecode(response['project_json']! as String)
                  as Map<String, Object?>;
          (project['meta']! as Map)['name'] = 'Smuggled';
          response['project_json'] = jsonEncode(project);
        },
        (response) {
          final project =
              jsonDecode(response['project_json']! as String)
                  as Map<String, Object?>;
          final entities = (project['entities']! as Map)
              .cast<String, Object?>();
          final slot =
              (entities[revision3DialogVoiceSlotCreationSlotId]! as Map)
                  .cast<String, Object?>();
          slot['display_name'] = 'Caller-controlled';
          response['project_json'] = jsonEncode(project);
        },
      ]) {
        final response = fixture.response();
        mutate(response);
        await expectLater(
          ModFfi(
            FakeGoreCoreFfiService(
              responses: {
                'authoring_store_prepare_revision3_dialog_voice_slot_creation_v1':
                    response,
              },
            ),
          ).authoringStorePrepareRevision3DialogVoiceSlotCreationV1(
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

Future<AuthoringRevision3DialogVoiceSlotCreationPreparation> _prepare(
  Revision3DialogVoiceSlotCreationFixture fixture,
) =>
    ModFfi(
      FakeGoreCoreFfiService(
        responses: {
          'authoring_store_prepare_revision3_dialog_voice_slot_creation_v1':
              fixture.response(),
        },
      ),
    ).authoringStorePrepareRevision3DialogVoiceSlotCreationV1(
      root: _root,
      currentProjectJson: fixture.projectJson,
      request: fixture.request(),
    );
