import 'dart:convert';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_quest_outline_fixture.dart';

const _root = r'C:\Projects\QuestOutline.goreproj';

void main() {
  test('Studio handshake requires the sorted Quest outline command', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_prepare_revision3_quest_outline_edit_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('request is exact-project bound and carries outline intent only', () {
    final fixture = Revision3QuestOutlineFixture();
    final request = fixture.request();
    final wire = jsonDecode(request.canonicalJson) as Map<String, Object?>;

    expect(wire.keys, <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_target',
      'quest_id',
      'expected_quest_revision',
      'display_name',
      'title',
      'objective_titles',
    ]);
    expect(wire, isNot(contains('module_id')));
    expect(wire, isNot(contains('game_root')));
    expect(wire, isNot(contains('collision_catalog')));
    expect(request.moduleId, revision3QuestOutlineModuleId);
    expect(request.expectedModuleRevision, 5);
  });

  test('request rejects count changes, whitespace and no-op edits', () {
    final fixture = Revision3QuestOutlineFixture();
    expect(
      () => fixture.request(objectiveTitles: const ['Only one']),
      throwsFormatException,
    );
    expect(
      () => fixture.request(displayName: ' Padded name '),
      throwsFormatException,
    );
    expect(
      () => fixture.request(
        displayName: fixture.displayName,
        title: fixture.title,
        objectiveTitles: fixture.objectiveTitles,
      ),
      throwsFormatException,
    );
  });

  test('FFI sends no game root and accepts exact sealed delta', () async {
    final fixture = Revision3QuestOutlineFixture();
    final request = fixture.request();
    final basisProjectBytes = utf8.encode(fixture.projectJson);
    final basisProjectSha = crypto.sha256.convert(basisProjectBytes).toString();
    expect(fixture.head.snapshotByteLength, isNot(basisProjectBytes.length));
    expect(fixture.head.snapshotSha256, isNot(basisProjectSha));
    final core = FakeGoreCoreFfiService(
      responses: {
        'authoring_store_prepare_revision3_quest_outline_edit_v1': fixture
            .response(),
      },
    );

    final prepared = await ModFfi(core)
        .authoringStorePrepareRevision3QuestOutlineEditV1(
          root: _root,
          currentProjectJson: fixture.projectJson,
          request: request,
        );

    expect(prepared.projectId, revision3QuestOutlineProjectId);
    expect(prepared.revision, 8);
    expect(prepared.questId, revision3QuestOutlineQuestId);
    expect(prepared.moduleId, revision3QuestOutlineModuleId);
    expect(prepared.questRevision, 5);
    expect(prepared.moduleRevision, 6);
    final candidateProjectBytes = utf8.encode(prepared.projectJson);
    final candidateProjectSha = crypto.sha256
        .convert(candidateProjectBytes)
        .toString();
    expect(
      prepared.head.snapshotByteLength,
      isNot(candidateProjectBytes.length),
    );
    expect(prepared.head.snapshotSha256, isNot(candidateProjectSha));
    expect(
      prepared.buildStatus,
      AuthoringRevision3QuestOutlineBuildStatus.blocked,
    );
    expect(
      prepared.runtimeStatus,
      AuthoringRevision3QuestOutlineRuntimeStatus.runtimeUnqualified,
    );
    expect(
      prepared.publicationStatus,
      AuthoringRevision3QuestOutlinePublicationStatus.notSupported,
    );
    final call = core.calls.single;
    expect(
      call.command,
      'authoring_store_prepare_revision3_quest_outline_edit_v1',
    );
    expect(call.payload.keys, <String>[
      'current_project_json',
      'quest_outline_request_json',
      'root',
    ]);
  });

  test(
    'FFI rejects an unrelated candidate delta as malformed native data',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final response = fixture.response();
      final candidate =
          jsonDecode(response['project_json']! as String)
              as Map<String, Object?>;
      (candidate['meta']! as Map<String, Object?>)['name'] = 'Smuggled change';
      final candidateJson = jsonEncode(candidate);
      response['project_json'] = candidateJson;
      response['head_json'] = headFor(candidateJson).canonicalJson;

      await expectLater(
        ModFfi(
          FakeGoreCoreFfiService(
            responses: {
              'authoring_store_prepare_revision3_quest_outline_edit_v1':
                  response,
            },
          ),
        ).authoringStorePrepareRevision3QuestOutlineEditV1(
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
}
