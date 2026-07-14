import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '../support/revision3_quest_outline_fixture.dart';

const _root = r'C:\Projects\QuestContext.goreproj';
const _gameRoot = r'C:\Games\Gothic 1 Remake';

void main() {
  test('Studio handshake requires the sorted Quest context command', () {
    expect(
      requiredStudioCoreCommands,
      contains('authoring_store_prepare_revision3_quest_context_edit_v1'),
    );
    expect(
      requiredStudioCoreCommands,
      orderedEquals(<String>[...requiredStudioCoreCommands]..sort()),
    );
  });

  test('private seed binds the exact visible Quest and generated module', () {
    final fixture = Revision3QuestOutlineFixture();
    final seed = AuthoringRevision3QuestContextSeed.forProject(
      currentProjectJson: fixture.projectJson,
      questId: revision3QuestOutlineQuestId,
      expectedQuestRevision: fixture.questRevision,
      expectedModuleId: revision3QuestOutlineModuleId,
      expectedModuleRevision: fixture.moduleRevision,
      expectedParentRuntimeClass: 'UQuest_SwampCamp_SCChapter2',
      expectedGiverRuntimeUniqueName: 'OM_GRD_Asghan_263',
    );

    expect(seed.projectId, revision3QuestOutlineProjectId);
    expect(seed.projectRevision, fixture.projectRevision);
    expect(
      seed.description,
      'Find the missing worker without changing runtime logic.',
    );
    expect(seed.parentRuntimeClass, 'UQuest_SwampCamp_SCChapter2');
    expect(seed.giverRuntimeUniqueName, 'OM_GRD_Asghan_263');
    expect(
      () => AuthoringRevision3QuestContextSeed.forProject(
        currentProjectJson: fixture.projectJson,
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixture.questRevision + 1,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: fixture.moduleRevision,
        expectedParentRuntimeClass: 'UQuest_SwampCamp_SCChapter2',
        expectedGiverRuntimeUniqueName: 'OM_GRD_Asghan_263',
      ),
      throwsFormatException,
    );
  });

  test('request has the exact canonical authority-minimal field set', () {
    final fixture = Revision3QuestOutlineFixture();
    final request = fixture.contextRequest();
    final wire = jsonDecode(request.canonicalJson) as Map<String, Object?>;

    expect(wire.keys, <String>[
      'expected_head',
      'expected_project_id',
      'expected_revision',
      'expected_story_catalog_seal',
      'quest_id',
      'expected_quest_revision',
      'description',
      'parent_catalog_id',
      'giver_catalog_id',
    ]);
    expect(wire, isNot(contains('module_id')));
    expect(wire, isNot(contains('game_root')));
    expect(
      wire.values,
      isNot(contains(revision3QuestContextParentRuntimeClass)),
    );
    expect(
      wire.values,
      isNot(contains(revision3QuestContextGiverRuntimeUniqueName)),
    );
    expect(request.moduleId, revision3QuestOutlineModuleId);
    expect(request.expectedModuleRevision, fixture.moduleRevision);
  });

  test(
    'FFI preserves exact outer payload and accepts only the sealed delta',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final core = FakeGoreCoreFfiService(
        responses: {
          'authoring_store_prepare_revision3_quest_context_edit_v1': fixture
              .contextResponse(),
        },
      );

      final prepared = await ModFfi(core)
          .authoringStorePrepareRevision3QuestContextEditV1(
            root: _root,
            gameRoot: _gameRoot,
            currentProjectJson: fixture.projectJson,
            request: fixture.contextRequest(),
          );

      expect(prepared.projectId, revision3QuestOutlineProjectId);
      expect(prepared.revision, fixture.projectRevision + 1);
      expect(prepared.questId, revision3QuestOutlineQuestId);
      expect(prepared.moduleId, revision3QuestOutlineModuleId);
      expect(prepared.questRevision, fixture.questRevision + 1);
      expect(prepared.moduleRevision, fixture.moduleRevision + 1);
      expect(prepared.parentCatalogId, revision3QuestContextParentCatalogId);
      expect(prepared.giverCatalogId, revision3QuestContextGiverCatalogId);
      expect(
        prepared.parentRuntimeClass,
        revision3QuestContextParentRuntimeClass,
      );
      expect(
        prepared.giverRuntimeUniqueName,
        revision3QuestContextGiverRuntimeUniqueName,
      );
      expect(
        prepared.buildStatus,
        AuthoringRevision3QuestContextBuildStatus.blocked,
      );
      expect(
        prepared.runtimeStatus,
        AuthoringRevision3QuestContextRuntimeStatus.runtimeUnqualified,
      );
      expect(
        prepared.publicationStatus,
        AuthoringRevision3QuestContextPublicationStatus.notSupported,
      );
      final call = core.calls.single;
      expect(
        call.command,
        'authoring_store_prepare_revision3_quest_context_edit_v1',
      );
      expect(call.payload.keys, <String>[
        'current_project_json',
        'game_root',
        'quest_context_request_json',
        'root',
      ]);
      expect(call.payload['game_root'], _gameRoot);
    },
  );

  test(
    'FFI rejects unrelated or authority-widening native responses',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final response = fixture.contextResponse();
      final candidate =
          jsonDecode(response['project_json']! as String)
              as Map<String, Object?>;
      (candidate['meta']! as Map<String, Object?>)['author'] = 'smuggled';
      final candidateJson = jsonEncode(candidate);
      response['project_json'] = candidateJson;
      response['head_json'] = headFor(candidateJson).canonicalJson;

      await expectLater(
        ModFfi(
          FakeGoreCoreFfiService(
            responses: {
              'authoring_store_prepare_revision3_quest_context_edit_v1':
                  response,
            },
          ),
        ).authoringStorePrepareRevision3QuestContextEditV1(
          root: _root,
          gameRoot: _gameRoot,
          currentProjectJson: fixture.projectJson,
          request: fixture.contextRequest(),
        ),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            ModFfiException.malformedNativeResponseCode,
          ),
        ),
      );

      final widened = fixture.contextResponse()
        ..['runtime_status'] = 'runtime_ready';
      await expectLater(
        ModFfi(
          FakeGoreCoreFfiService(
            responses: {
              'authoring_store_prepare_revision3_quest_context_edit_v1':
                  widened,
            },
          ),
        ).authoringStorePrepareRevision3QuestContextEditV1(
          root: _root,
          gameRoot: _gameRoot,
          currentProjectJson: fixture.projectJson,
          request: fixture.contextRequest(),
        ),
        throwsA(isA<ModFfiException>()),
      );
    },
  );

  test(
    'FFI rejects same-runtime candidates with changed hidden provenance',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      for (final response in <Map<String, Object?>>[
        fixture.contextResponse(
          parentCatalogLayer: 'base-game.quest-parent.wrong',
        ),
        fixture.contextResponse(
          parentAuthoringSelector: 'WrongSameRuntimeSelector',
        ),
        fixture.contextResponse(
          parentSourceSeal: <String, Object?>{
            'byte_len': 99,
            'sha256': List<String>.filled(64, '8').join(),
          },
        ),
      ]) {
        await expectLater(
          ModFfi(
            FakeGoreCoreFfiService(
              responses: {
                'authoring_store_prepare_revision3_quest_context_edit_v1':
                    response,
              },
            ),
          ).authoringStorePrepareRevision3QuestContextEditV1(
            root: _root,
            gameRoot: _gameRoot,
            currentProjectJson: fixture.projectJson,
            request: fixture.contextRequest(),
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
