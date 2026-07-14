import 'dart:async';
import 'dart:collection';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/managed_project_lock.dart';
import 'package:gore_mod/project/managed_project_session.dart';
import 'package:gore_mod/project/project_atomic_io.dart';
import 'package:path/path.dart' as p;

import '../dataasset/dataasset_test_fixtures.dart';
import '../support/revision3_dataasset_fixture.dart';
import '../support/revision3_npc_fixture.dart';
import '../support/revision3_quest_fixture.dart';
import '../support/revision3_quest_outline_fixture.dart';
import '../support/revision3_voice_fixture.dart';
import '../support/revision3_voice_selection_fixture.dart';

void main() {
  late Directory fixture;

  setUp(() async {
    fixture = await Directory.systemTemp.createTemp(
      'gore_managed_revision3_session_',
    );
  });

  tearDown(() async {
    if (await fixture.exists()) await fixture.delete(recursive: true);
  });

  test(
    'production adapter uses only the dedicated revision-3 commands',
    () async {
      final project = _projectJson(revision: 7, name: 'Adapter');
      final fixtureStore = _FakeRevision3Store();
      final head = fixtureStore.register(project);
      final questRequest = AuthoringRevision3QuestDraftRequestV3(
        expectedHead: head,
        expectedProjectId: '00000000000000000000000000000003',
        expectedRevision: 7,
        questId: '00000000000000000000000000000071',
        scriptModuleId: '00000000000000000000000000000072',
        displayName: 'Managed Quest 1',
        intent: _questIntent(1),
      );
      final projectMap = jsonDecode(project) as Map<String, Object?>;
      final questInput = _questInput(
        request: questRequest,
        basisHead: head,
        target: (projectMap['target'] as Map).cast<String, Object?>(),
      );
      final candidateMap = jsonDecode(project) as Map<String, Object?>
        ..['revision'] = 8
        ..['entities'] = <String, Object?>{
          questRequest.questId: _questEntity(
            projectId: questRequest.expectedProjectId,
            request: questRequest,
            input: questInput,
          ),
          questRequest.scriptModuleId: _questModuleEntity(
            projectId: questRequest.expectedProjectId,
            request: questRequest,
            input: questInput,
          ),
        }
        ..['asset_store'] = <String, Object?>{
          'assets': <String, Object?>{
            _questArtifactSha: <String, Object?>{
              'byte_len': 123,
              'media_type':
                  'application/vnd.gore.quest-collision-capability+json;version=2',
            },
          },
        };
      final candidateProject = jsonEncode(candidateMap);
      final candidateHead = fixtureStore.register(candidateProject);
      final npcRequest = AuthoringRevision3NpcDraftRequestV1.forProject(
        expectedHead: head,
        currentProjectJson: project,
        npcId: '00000000000000000000000000000081',
        scriptModuleId: '00000000000000000000000000000082',
        displayName: 'Managed Guard',
        intent: _npcIntent(1),
      );
      final npcFixture = Revision3NpcFixture.fromBasis(
        basisHead: head,
        basisProjectJson: project,
        request: npcRequest,
      );
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_open_revision3': _openedResponse(head, project),
          'authoring_store_prepare_revision3_checkpoint': _preparedResponse(
            head,
          ),
          'authoring_store_open_revision3_head_bytes': _openedResponse(
            head,
            project,
          ),
          'authoring_store_read_revision3_content_index_v1': _contentResponse(
            head,
            project,
          ),
          'authoring_store_prepare_revision3_quest_draft_v3':
              _questPreparedResponse(
                basisHead: head,
                candidateHead: candidateHead,
                candidateProjectJson: candidateProject,
                revision: 8,
                questId: questRequest.questId,
                scriptModuleId: questRequest.scriptModuleId,
              ),
          'authoring_store_prepare_revision3_npc_draft_v1': npcFixture
              .response(),
        },
      );
      final adapter = ModFfiManagedRevision3AuthoringStore(ModFfi(core));

      await adapter.open(
        root: fixture.path,
        verification: AuthoringAssetVerification.full,
      );
      await adapter.prepareCheckpoint(
        root: fixture.path,
        expectedHead: head,
        projectJson: project,
      );
      await adapter.openHeadBytes(
        root: fixture.path,
        head: head,
        verification: AuthoringAssetVerification.full,
      );
      await adapter.readContentIndex(root: fixture.path, expectedHead: head);
      await adapter.prepareQuestDraftV3(
        root: fixture.path,
        gameRoot: r'D:\Games\Gothic Remake',
        currentProjectJson: project,
        questRequestJson: questRequest.canonicalJson,
      );
      await adapter.prepareNpcDraftV1(
        root: fixture.path,
        gameRoot: r'D:\Games\Gothic Remake',
        currentProjectJson: project,
        request: npcRequest,
      );

      expect(core.calls.map((call) => call.command), <String>[
        'authoring_store_open_revision3',
        'authoring_store_prepare_revision3_checkpoint',
        'authoring_store_open_revision3_head_bytes',
        'authoring_store_read_revision3_content_index_v1',
        'authoring_store_prepare_revision3_quest_draft_v3',
        'authoring_store_prepare_revision3_npc_draft_v1',
      ]);
      expect(core.calls[0].payload, <String, Object?>{
        'root': fixture.path,
        'verification': 'full',
      });
      expect(core.calls[1].payload, <String, Object?>{
        'root': fixture.path,
        'expected_head_json': head.canonicalJson,
        'project_json': project,
      });
      expect(core.calls[2].payload, <String, Object?>{
        'root': fixture.path,
        'head_json': head.canonicalJson,
        'verification': 'full',
      });
      expect(core.calls[3].payload, <String, Object?>{
        'expected_head_json': head.canonicalJson,
        'root': fixture.path,
      });
      expect(core.calls[4].payload, <String, Object?>{
        'current_project_json': project,
        'game_root': r'D:\Games\Gothic Remake',
        'quest_request_json': questRequest.canonicalJson,
        'root': fixture.path,
      });
      expect(core.calls[5].payload, <String, Object?>{
        'current_project_json': project,
        'game_root': r'D:\Games\Gothic Remake',
        'npc_request_json': npcRequest.canonicalJson,
        'root': fixture.path,
      });
    },
  );

  test(
    'create, save, derive, close, and open preserve exact R3 bytes',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'Original');
      final saved = _projectJson(revision: 1, name: 'Saved');
      final derived = _projectJson(revision: 2, name: 'Derived');

      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      expect(session.projectJson, original);
      expect(session.projectId, '00000000000000000000000000000003');
      expect(session.projectRevision, 0);
      expect(store.expectedHeads, <String?>[null]);
      expect(await session.headFile.readAsString(), session.head.canonicalJson);

      final firstHead = session.head.canonicalJson;
      await session.save(saved);
      expect(session.projectJson, saved);
      expect(session.projectRevision, 1);
      expect(store.expectedHeads[1], firstHead);

      final value = await session.deriveAndSave<String>((latest) {
        expect(latest, saved);
        return ManagedProjectDerivedCandidate<String>(
          projectJson: derived,
          value: 'published',
        );
      });
      expect(value, 'published');
      expect(session.projectJson, derived);
      expect(session.projectRevision, 2);
      final exactFinalHead = session.head.canonicalJson;
      await session.close();
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.head.canonicalJson, exactFinalHead);
      expect(reopened.projectJson, derived);
      expect(reopened.projectRevision, 2);
      expect(
        store.openVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      await reopened.close();
    },
  );

  test(
    'queued Quest transactions bind latest R3 basis and publish fully reopened candidates',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'Quest project');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      final originalHead = session.head.canonicalJson;
      final genericPrepares = store.prepareCalls;

      final first = session.prepareAndPublishQuestDraftV3(
        gameRoot: r'D:\Games\Gothic Remake',
        questId: '00000000000000000000000000000071',
        scriptModuleId: '00000000000000000000000000000072',
        displayName: 'Managed Quest 1',
        intent: _questIntent(
          1,
          additionalObjectiveTitles: const <String>[
            'Inspect the gate',
            'Report to Asghan',
          ],
        ),
      );
      final second = session.prepareAndPublishQuestDraftV3(
        gameRoot: r'D:\Games\Gothic Remake',
        questId: '00000000000000000000000000000073',
        scriptModuleId: '00000000000000000000000000000074',
        displayName: 'Managed Quest 2',
        intent: _questIntent(2),
      );
      final results = await Future.wait(
        <Future<ManagedRevision3QuestDraftCheckpoint>>[first, second],
      );

      expect(store.prepareCalls, genericPrepares);
      expect(store.questPrepareCalls, 2);
      expect(store.questCurrentProjects[0], original);
      expect(store.questRequests[0].expectedHead.canonicalJson, originalHead);
      expect(store.questRequests[0].expectedRevision, 0);
      expect(store.questRequests[0].intent.additionalObjectiveTitles, <String>[
        'Inspect the gate',
        'Report to Asghan',
      ]);
      expect(
        results[0].projectJson,
        contains(
          '"additional_objective_titles":["Inspect the gate","Report to Asghan"]',
        ),
      );
      expect(
        store.questRequests[1].expectedHead.canonicalJson,
        results[0].head.canonicalJson,
      );
      expect(store.questRequests[1].expectedRevision, 1);
      expect(store.questGameRoots, <String>[
        r'D:\Games\Gothic Remake',
        r'D:\Games\Gothic Remake',
      ]);
      expect(results[0].projectRevision, 1);
      expect(results[0].questId, '00000000000000000000000000000071');
      expect(results[1].projectRevision, 2);
      expect(results[1].questId, '00000000000000000000000000000073');
      expect(session.projectJson, results[1].projectJson);
      expect(session.projectRevision, 2);
      expect(session.head.canonicalJson, results[1].head.canonicalJson);
      expect(
        await session.headFile.readAsString(),
        results[1].head.canonicalJson,
      );
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      expect(
        store.openVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, results[1].projectJson);
      expect(reopened.projectRevision, 2);
      await reopened.close();
    },
  );

  test(
    'queued NPC transactions bind latest R3 basis and publish fully reopened candidates',
    () async {
      final root = await _projectRoot(fixture, suffix: 'npc_queue');
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'NPC project');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      final originalHead = session.head.canonicalJson;
      final genericPrepares = store.prepareCalls;

      final first = session.prepareAndPublishNpcDraftV1(
        gameRoot: r'D:\Games\Gothic Remake',
        npcId: '00000000000000000000000000000081',
        scriptModuleId: '00000000000000000000000000000082',
        displayName: 'Managed Guard 1',
        intent: _npcIntent(1),
      );
      final second = session.prepareAndPublishNpcDraftV1(
        gameRoot: r'D:\Games\Gothic Remake',
        npcId: '00000000000000000000000000000083',
        scriptModuleId: '00000000000000000000000000000084',
        displayName: 'Managed Guard 2',
        intent: _npcIntent(2),
      );
      final results = await Future.wait(
        <Future<ManagedRevision3NpcDraftCheckpoint>>[first, second],
      );

      expect(store.prepareCalls, genericPrepares);
      expect(store.npcPrepareCalls, 2);
      expect(store.npcCurrentProjects[0], original);
      expect(store.npcRequests[0].expectedHead.canonicalJson, originalHead);
      expect(store.npcRequests[0].expectedRevision, 0);
      expect(
        store.npcRequests[1].expectedHead.canonicalJson,
        results[0].head.canonicalJson,
      );
      expect(store.npcRequests[1].expectedRevision, 1);
      expect(store.npcGameRoots, <String>[
        r'D:\Games\Gothic Remake',
        r'D:\Games\Gothic Remake',
      ]);
      expect(results[0].projectRevision, 1);
      expect(results[0].npcId, '00000000000000000000000000000081');
      expect(results[0].uniqueName, 'GoreManagedNpc1');
      expect(results[1].projectRevision, 2);
      expect(results[1].npcId, '00000000000000000000000000000083');
      expect(results[1].parentCatalogId, 'g1r:npc:om_grd_asghan_263');
      expect(session.projectJson, results[1].projectJson);
      expect(session.projectRevision, 2);
      expect(session.head.canonicalJson, results[1].head.canonicalJson);
      expect(
        await session.headFile.readAsString(),
        results[1].head.canonicalJson,
      );
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      expect(
        store.openVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, results[1].projectJson);
      expect(reopened.projectRevision, 2);
      await reopened.close();
    },
  );

  test(
    'Voice transaction binds the exact lane and publishes only after two full reopens',
    () async {
      final root = await _projectRoot(fixture, suffix: 'voice_publish');
      final store = _FakeRevision3Store();
      final original = revision3VoiceFixtureProjectJson();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      final basisHead = session.head.canonicalJson;
      final genericPrepares = store.prepareCalls;

      final result = await session.prepareAndPublishVoiceTakeV1(
        gameRoot: r'D:\Games\Gothic Remake',
        source: r'D:\Recordings\asghan.ogg',
        lineId: revision3VoiceFixtureLineId,
        slotId: revision3VoiceFixtureSlotId,
        takeId: revision3VoiceFixtureTakeId,
        locale: 'de',
        takeDisplayName: 'Asghan DE Take 1',
        logicalName: 'GRD_263_ASGHAN_OPEN_INFO_06_02.ogg',
        status: AuthoringRevision3VoiceTakeStatus.recorded,
      );

      expect(store.prepareCalls, genericPrepares);
      expect(store.voicePrepareCalls, 1);
      expect(store.voiceGameRoots, <String>[r'D:\Games\Gothic Remake']);
      expect(store.voiceSources, <String>[r'D:\Recordings\asghan.ogg']);
      expect(store.voiceCurrentProjects, <String>[original]);
      expect(store.voiceRequests.single.expectedHead.canonicalJson, basisHead);
      expect(store.voiceRequests.single.expectedRevision, 7);
      expect(result.projectRevision, 8);
      expect(result.projectId, revision3VoiceFixtureProjectId);
      expect(result.localizationId, revision3VoiceFixtureLocalizationId);
      expect(result.takeId, revision3VoiceFixtureTakeId);
      expect(result.slotCreated, isTrue);
      expect(result.selected, isFalse);
      expect(result.asset.sha256, revision3VoiceFixtureAssetSha256);
      expect(session.projectJson, result.projectJson);
      expect(session.head.canonicalJson, result.head.canonicalJson);
      expect(await session.headFile.readAsString(), result.head.canonicalJson);
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      expect(
        store.openVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, result.projectJson);
      expect(reopened.projectRevision, 8);
      await reopened.close();
    },
  );

  test(
    'Voice target publishes sealed archive evidence and build is an exact non-publishing read',
    () async {
      final root = await _projectRoot(fixture, suffix: 'voice_target_build');
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: revision3VoiceFixtureProjectJson(),
      );
      await session.prepareAndPublishVoiceTakeV1(
        gameRoot: r'D:\Games\Gothic Remake',
        source: r'D:\Recordings\asghan.ogg',
        lineId: revision3VoiceFixtureLineId,
        slotId: revision3VoiceFixtureSlotId,
        takeId: revision3VoiceFixtureTakeId,
        locale: 'de',
        takeDisplayName: 'Asghan approved take',
        logicalName: 'GRD_263_ASGHAN_OPEN_INFO_06_02.ogg',
        status: AuthoringRevision3VoiceTakeStatus.approved,
        selectTake: true,
      );

      final target = await session.prepareAndPublishVoiceTargetV1(
        gameRoot: r'D:\Games\Gothic Remake',
        lineId: revision3VoiceFixtureLineId,
        slotId: revision3VoiceFixtureSlotId,
        locale: 'de',
        expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
      );
      expect(target.projectRevision, 9);
      expect(
        target.resolution,
        AuthoringRevision3VoiceTargetResolutionState.resolved,
      );
      expect(target.targets.single.archive, 'german_new.zip');
      expect(target.archiveObservation!.sha256, 'c' * 64);
      expect(store.voiceTargetPrepareCalls, 1);
      expect(store.voiceTargetRequests.single.expectedRevision, 8);
      expect(await session.headFile.readAsString(), target.head.canonicalJson);

      final fixedHead = await session.headFile.readAsBytes();
      final result = await session.buildVoiceV1(
        gameRoot: r'D:\Games\Gothic Remake',
        output: p.join(fixture.path, 'voice-bundle'),
      );
      expect(result.isBuilt, isTrue);
      expect(result.projectRevision, 9);
      expect(store.voiceBuildCalls, 1);
      expect(store.voiceBuildGameRoots, <String>[r'D:\Games\Gothic Remake']);
      expect(session.projectRevision, 9);
      expect(await session.headFile.readAsBytes(), orderedEquals(fixedHead));
      await session.close();
    },
  );

  test(
    'Voice target and build classify retryable output/input failures separately from Store integrity',
    () async {
      final retryRoot = await _projectRoot(
        fixture,
        suffix: 'voice_target_retry',
      );
      final retryStore = _FakeRevision3Store();
      final retrySession = await ManagedRevision3AuthoringProjectSession.create(
        root: retryRoot,
        store: retryStore,
        projectJson: revision3VoiceFixtureProjectJson(),
      );
      final retryHead = await retrySession.headFile.readAsBytes();
      retryStore.nextVoiceTargetError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_voice_target_v1',
        code: 'AUTHORING_REVISION3_VOICE_TARGET_ARCHIVE_UNAVAILABLE',
        message: 'fake archive unavailable',
      );
      await expectLater(
        retrySession.prepareAndPublishVoiceTargetV1(
          gameRoot: r'D:\Games\Gothic Remake',
          lineId: revision3VoiceFixtureLineId,
          slotId: revision3VoiceFixtureSlotId,
          locale: 'de',
          expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
        ),
        throwsA(isA<ModFfiException>()),
      );
      expect(retrySession.requiresReopen, isFalse);
      expect(
        await retrySession.headFile.readAsBytes(),
        orderedEquals(retryHead),
      );

      for (final code in <String>[
        'AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_FAILED',
        'AUTHORING_REVISION3_VOICE_BUILD_GAME_UNAVAILABLE',
        'AUTHORING_REVISION3_VOICE_BUILD_STORE_GAME_ALIAS',
        'AUTHORING_REVISION3_VOICE_BUILD_GAME_OUTPUT_ALIAS',
        'AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_UNAVAILABLE',
        'AUTHORING_REVISION3_VOICE_BUILD_EXECUTABLE_MISMATCH',
        'AUTHORING_REVISION3_VOICE_BUILD_PROMOTION_FAILED',
        'AUTHORING_REVISION3_VOICE_BUILD_CLEANUP_FAILED',
        'AUTHORING_REVISION3_VOICE_BUILD_GAME_ROOT_CHANGED',
        'AUTHORING_REVISION3_VOICE_BUILD_OUTPUT_ROOT_CHANGED',
      ]) {
        retryStore.nextVoiceBuildError = ModFfiException(
          command: 'authoring_store_build_revision3_voice_v1',
          code: code,
          message: 'fake retryable Voice build failure',
        );
        await expectLater(
          retrySession.buildVoiceV1(
            gameRoot: r'D:\Games\Gothic Remake',
            output: p.join(fixture.path, 'exists'),
          ),
          throwsA(isA<ModFfiException>()),
        );
        expect(retrySession.requiresReopen, isFalse, reason: code);
        expect(
          await retrySession.headFile.readAsBytes(),
          orderedEquals(retryHead),
          reason: code,
        );
      }

      retryStore.nextVoiceBuildError = const ModFfiException(
        command: 'authoring_store_build_revision3_voice_v1',
        code: 'AUTHORING_REVISION3_VOICE_BUILD_PUBLICATION_UNCONFIRMED',
        message: 'fake ambiguous output publication',
      );
      await expectLater(
        retrySession.buildVoiceV1(
          gameRoot: r'D:\Games\Gothic Remake',
          output: p.join(fixture.path, 'ambiguous-output'),
        ),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            'AUTHORING_REVISION3_VOICE_BUILD_PUBLICATION_UNCONFIRMED',
          ),
        ),
      );
      expect(retrySession.requiresReopen, isFalse);
      await retrySession.close();

      final poisonRoot = await _projectRoot(
        fixture,
        suffix: 'voice_target_poison',
      );
      final poisonStore = _FakeRevision3Store();
      final poisonSession =
          await ManagedRevision3AuthoringProjectSession.create(
            root: poisonRoot,
            store: poisonStore,
            projectJson: revision3VoiceFixtureProjectJson(),
          );
      poisonStore.nextVoiceTargetError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_voice_target_v1',
        code: 'AUTHORING_REVISION3_VOICE_TARGET_STORE_SEAL_MISMATCH',
        message: 'fake Store integrity failure',
      );
      await expectLater(
        poisonSession.prepareAndPublishVoiceTargetV1(
          gameRoot: r'D:\Games\Gothic Remake',
          lineId: revision3VoiceFixtureLineId,
          slotId: revision3VoiceFixtureSlotId,
          locale: 'de',
          expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(poisonSession.requiresReopen, isTrue);
      await poisonSession.close();

      final buildPoisonRoot = await _projectRoot(
        fixture,
        suffix: 'voice_build_store_root_poison',
      );
      final buildPoisonStore = _FakeRevision3Store();
      final buildPoisonSession =
          await ManagedRevision3AuthoringProjectSession.create(
            root: buildPoisonRoot,
            store: buildPoisonStore,
            projectJson: revision3VoiceFixtureProjectJson(),
          );
      buildPoisonStore.nextVoiceBuildError = const ModFfiException(
        command: 'authoring_store_build_revision3_voice_v1',
        code: 'AUTHORING_REVISION3_VOICE_BUILD_STORE_ROOT_CHANGED',
        message: 'fake Store-root identity drift',
      );
      await expectLater(
        buildPoisonSession.buildVoiceV1(
          gameRoot: r'D:\Games\Gothic Remake',
          output: p.join(fixture.path, 'store-drift-output'),
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(buildPoisonSession.requiresReopen, isTrue);
      await buildPoisonSession.close();
    },
  );

  test(
    'Voice target publication preserves an intact full 1024-candidate slot',
    () async {
      final root = await _projectRoot(fixture, suffix: 'voice_target_full');
      final store = _FakeRevision3Store();
      final fullProject = revision3VoiceFixtureProjectWithExistingSlotJson(
        candidateCount: 1024,
      );
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: fullProject,
      );

      final target = await session.prepareAndPublishVoiceTargetV1(
        gameRoot: r'D:\Games\Gothic Remake',
        lineId: revision3VoiceFixtureLineId,
        slotId: revision3VoiceFixtureSlotId,
        locale: 'de',
        expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
      );

      expect(target.projectRevision, 9);
      expect(
        target.resolution,
        AuthoringRevision3VoiceTargetResolutionState.resolved,
      );
      expect(store.voiceTargetPrepareCalls, 1);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'Voice build returns its basis receipt and marks reopen after a later head drift',
    () async {
      final root = await _projectRoot(fixture, suffix: 'voice_build_snapshot');
      final store = _FakeRevision3Store();
      final basisProject = revision3VoiceFixtureBuildReadyProjectJson();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: basisProject,
      );
      final externalProject = (jsonDecode(basisProject) as Map)
          .cast<String, Object?>();
      externalProject['revision'] = (externalProject['revision']! as int) + 1;
      final externalHead = store.register(jsonEncode(externalProject));
      store.afterVoiceBuild = (storeRoot, _) async {
        await File(
          p.join(storeRoot, 'gore-project.json'),
        ).writeAsString(externalHead.canonicalJson, flush: true);
      };

      final result = await session.buildVoiceV1(
        gameRoot: r'D:\Games\Gothic Remake',
        output: p.join(fixture.path, 'snapshot-voice-bundle'),
      );

      expect(result.isBuilt, isTrue);
      expect(result.projectRevision, session.projectRevision);
      expect(result.output, p.join(fixture.path, 'snapshot-voice-bundle'));
      expect(session.requiresReopen, isTrue);
      expect(await session.headFile.readAsString(), externalHead.canonicalJson);
      await expectLater(
        session.buildVoiceV1(
          gameRoot: r'D:\Games\Gothic Remake',
          output: p.join(fixture.path, 'another-voice-bundle'),
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      await session.close();
    },
  );

  test(
    'Voice local and source rejections retry without changing the exact head',
    () async {
      final root = await _projectRoot(fixture, suffix: 'voice_errors');
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: revision3VoiceFixtureProjectJson(),
      );
      final fixedHead = await session.headFile.readAsBytes();

      await expectLater(
        session.prepareAndPublishVoiceTakeV1(
          gameRoot: r'D:\Games\Gothic Remake',
          source: 'take.ogg',
          lineId: revision3VoiceFixtureLineId,
          slotId: revision3VoiceFixtureSlotId,
          takeId: revision3VoiceFixtureTakeId,
          locale: 'de',
          text: 'Du willst in die Mine?',
          takeDisplayName: 'Asghan DE Take 1',
          logicalName: 'asghan.ogg',
          status: AuthoringRevision3VoiceTakeStatus.reviewed,
          selectTake: true,
        ),
        throwsFormatException,
      );
      expect(store.voicePrepareCalls, 0);
      expect(session.requiresReopen, isFalse);
      expect(await session.headFile.readAsBytes(), orderedEquals(fixedHead));

      const retryableCodes = <String>[
        'AUTHORING_REVISION3_VOICE_GAME_ROOT_UNAVAILABLE',
        'AUTHORING_REVISION3_VOICE_STORE_GAME_ALIAS',
        'AUTHORING_REVISION3_VOICE_INPUT_MISSING',
        'AUTHORING_REVISION3_VOICE_INPUT_UNAVAILABLE',
        'AUTHORING_REVISION3_VOICE_INPUT_UNSAFE',
        'AUTHORING_REVISION3_VOICE_INPUT_LIMIT',
        'AUTHORING_REVISION3_VOICE_OGG_INVALID',
        'AUTHORING_REVISION3_VOICE_INPUT_CHANGED',
      ];
      for (final code in retryableCodes) {
        store.nextVoiceError = ModFfiException(
          command: 'authoring_store_prepare_revision3_voice_take_v1',
          code: code,
          message: 'fake retryable Voice source rejection',
        );
        await expectLater(
          session.prepareAndPublishVoiceTakeV1(
            gameRoot: r'D:\Games\Gothic Remake',
            source: 'take.ogg',
            lineId: revision3VoiceFixtureLineId,
            slotId: revision3VoiceFixtureSlotId,
            takeId: revision3VoiceFixtureTakeId,
            locale: 'de',
            takeDisplayName: 'Asghan DE Take 1',
            logicalName: 'asghan.ogg',
            status: AuthoringRevision3VoiceTakeStatus.recorded,
          ),
          throwsA(
            isA<ModFfiException>().having((error) => error.code, 'code', code),
          ),
          reason: code,
        );
        expect(session.requiresReopen, isFalse, reason: code);
        expect(
          await session.headFile.readAsBytes(),
          orderedEquals(fixedHead),
          reason: code,
        );
      }

      final result = await session.prepareAndPublishVoiceTakeV1(
        gameRoot: r'D:\Games\Gothic Remake',
        source: 'take.ogg',
        lineId: revision3VoiceFixtureLineId,
        slotId: revision3VoiceFixtureSlotId,
        takeId: revision3VoiceFixtureTakeId,
        locale: 'de',
        takeDisplayName: 'Asghan DE Take 1',
        logicalName: 'asghan.ogg',
        status: AuthoringRevision3VoiceTakeStatus.recorded,
      );
      expect(result.projectRevision, 8);
      expect(store.voicePrepareCalls, retryableCodes.length + 1);
      await session.close();
    },
  );

  test(
    'Voice collision, response, and Store invariants poison the session',
    () async {
      const poisonCodes = <String>[
        'AUTHORING_REVISION3_VOICE_COLLISION',
        'AUTHORING_REVISION3_VOICE_RESPONSE_LIMIT',
        'AUTHORING_REVISION3_VOICE_STORE_SEAL_MISMATCH',
      ];
      for (final code in poisonCodes) {
        final root = await _projectRoot(
          fixture,
          suffix: 'voice_poison_${code.toLowerCase()}',
        );
        final store = _FakeRevision3Store();
        final session = await ManagedRevision3AuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: revision3VoiceFixtureProjectJson(),
        );
        final fixedHead = await session.headFile.readAsBytes();
        store.nextVoiceError = ModFfiException(
          command: 'authoring_store_prepare_revision3_voice_take_v1',
          code: code,
          message: 'fake non-retryable Voice invariant',
        );

        await expectLater(
          session.prepareAndPublishVoiceTakeV1(
            gameRoot: r'D:\Games\Gothic Remake',
            source: 'take.ogg',
            lineId: revision3VoiceFixtureLineId,
            slotId: revision3VoiceFixtureSlotId,
            takeId: revision3VoiceFixtureTakeId,
            locale: 'de',
            takeDisplayName: 'Asghan DE Take 1',
            logicalName: 'asghan.ogg',
            status: AuthoringRevision3VoiceTakeStatus.recorded,
          ),
          throwsA(isA<ManagedProjectVerificationException>()),
          reason: code,
        );
        expect(session.requiresReopen, isTrue, reason: code);
        expect(
          await session.headFile.readAsBytes(),
          orderedEquals(fixedHead),
          reason: code,
        );
        await session.close();
      }
    },
  );

  test(
    'Voice exhausted revision counter poisons the managed session',
    () async {
      final root = await _projectRoot(fixture, suffix: 'voice_revision_limit');
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: revision3VoiceFixtureProjectJson(),
      );
      final fixedHead = await session.headFile.readAsBytes();
      store.nextVoiceError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_voice_take_v1',
        code: 'AUTHORING_REVISION3_VOICE_REVISION_LIMIT',
        message: 'fake exhausted revision counter',
      );

      await expectLater(
        session.prepareAndPublishVoiceTakeV1(
          gameRoot: r'D:\Games\Gothic Remake',
          source: 'take.ogg',
          lineId: revision3VoiceFixtureLineId,
          slotId: revision3VoiceFixtureSlotId,
          takeId: revision3VoiceFixtureTakeId,
          locale: 'de',
          takeDisplayName: 'Asghan DE Take 1',
          logicalName: 'asghan.ogg',
          status: AuthoringRevision3VoiceTakeStatus.recorded,
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(store.voicePrepareCalls, 1);
      expect(session.requiresReopen, isTrue);
      expect(await session.headFile.readAsBytes(), orderedEquals(fixedHead));
      await session.close();
    },
  );

  test(
    'Voice selection and clear publish through exact full-reopen CAS without a game root',
    () async {
      final voice = Revision3VoiceSelectionFixture();
      final root = await _projectRoot(fixture, suffix: 'voice_selection');
      final store = _FakeRevision3Store(sealRegisteredHeads: true);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: voice.projectJson,
      );

      final selected = await session.prepareAndPublishVoiceTakeSelectionV1(
        lineId: revision3VoiceFixtureLineId,
        slotId: revision3VoiceFixtureSlotId,
        expectedSlotRevision: voice.slotRevision,
        locale: 'de',
        expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
        expectedSelectedTakeId: revision3VoiceFixtureTakeId,
        selectedTakeId: revision3VoiceSelectionAlternateTakeId,
      );

      expect(store.voiceSelectionPrepareCalls, 1);
      expect(store.voiceSelectionRequests.single.expectedRevision, 8);
      expect(selected.projectRevision, 9);
      expect(selected.slotRevision, voice.slotRevision + 1);
      expect(selected.previousSelectedTakeId, revision3VoiceFixtureTakeId);
      expect(selected.selectedTakeId, revision3VoiceSelectionAlternateTakeId);
      expect(session.projectJson, selected.projectJson);
      expect(
        await session.headFile.readAsString(),
        selected.head.canonicalJson,
      );

      final cleared = await session.prepareAndPublishVoiceTakeSelectionV1(
        lineId: revision3VoiceFixtureLineId,
        slotId: revision3VoiceFixtureSlotId,
        expectedSlotRevision: selected.slotRevision,
        locale: 'de',
        expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
        expectedSelectedTakeId: revision3VoiceSelectionAlternateTakeId,
        selectedTakeId: null,
      );
      expect(cleared.projectRevision, 10);
      expect(cleared.slotRevision, selected.slotRevision + 1);
      expect(cleared.selectedTakeId, isNull);
      expect(store.voiceSelectionPrepareCalls, 2);
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );

      await session.close();
      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectRevision, 10);
      expect(reopened.projectJson, cleared.projectJson);
      await reopened.close();
    },
  );

  test(
    'Voice selection semantic failures retry while head and integrity failures poison',
    () async {
      final voice = Revision3VoiceSelectionFixture();
      final retryRoot = await _projectRoot(
        fixture,
        suffix: 'voice_selection_retry',
      );
      final retryStore = _FakeRevision3Store(sealRegisteredHeads: true);
      final retrySession = await ManagedRevision3AuthoringProjectSession.create(
        root: retryRoot,
        store: retryStore,
        projectJson: voice.projectJson,
      );
      final fixedHead = await retrySession.headFile.readAsBytes();
      retryStore.nextVoiceSelectionError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_voice_take_selection_v1',
        code: 'AUTHORING_REVISION3_VOICE_SELECTION_TAKE_NOT_APPROVED',
        message: 'fake bounded selection rejection',
      );
      await expectLater(
        retrySession.prepareAndPublishVoiceTakeSelectionV1(
          lineId: revision3VoiceFixtureLineId,
          slotId: revision3VoiceFixtureSlotId,
          expectedSlotRevision: voice.slotRevision,
          locale: 'de',
          expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
          expectedSelectedTakeId: revision3VoiceFixtureTakeId,
          selectedTakeId: revision3VoiceSelectionAlternateTakeId,
        ),
        throwsA(isA<ModFfiException>()),
      );
      expect(retrySession.requiresReopen, isFalse);
      expect(await retrySession.headFile.readAsBytes(), fixedHead);
      final published = await retrySession
          .prepareAndPublishVoiceTakeSelectionV1(
            lineId: revision3VoiceFixtureLineId,
            slotId: revision3VoiceFixtureSlotId,
            expectedSlotRevision: voice.slotRevision,
            locale: 'de',
            expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
            expectedSelectedTakeId: revision3VoiceFixtureTakeId,
            selectedTakeId: revision3VoiceSelectionAlternateTakeId,
          );
      expect(published.projectRevision, 9);
      await retrySession.close();

      for (final code in <String>[
        'AUTHORING_REVISION3_VOICE_SELECTION_HEAD_CONFLICT',
        'AUTHORING_REVISION3_VOICE_SELECTION_STORE_INVARIANT',
        ModFfiException.malformedNativeResponseCode,
      ]) {
        final root = await _projectRoot(
          fixture,
          suffix: 'voice_selection_poison_${code.hashCode}',
        );
        final store = _FakeRevision3Store(sealRegisteredHeads: true);
        final session = await ManagedRevision3AuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: voice.projectJson,
        );
        store.nextVoiceSelectionError = ModFfiException(
          command: 'authoring_store_prepare_revision3_voice_take_selection_v1',
          code: code,
          message: 'fake Voice selection integrity failure',
        );
        await expectLater(
          session.prepareAndPublishVoiceTakeSelectionV1(
            lineId: revision3VoiceFixtureLineId,
            slotId: revision3VoiceFixtureSlotId,
            expectedSlotRevision: voice.slotRevision,
            locale: 'de',
            expectedLocId: 'GRD_263_ASGHAN_OPEN_INFO_06_02',
            expectedSelectedTakeId: revision3VoiceFixtureTakeId,
            selectedTakeId: revision3VoiceSelectionAlternateTakeId,
          ),
          code.endsWith('HEAD_CONFLICT')
              ? throwsA(isA<ManagedProjectHeadConflictException>())
              : throwsA(isA<ManagedProjectVerificationException>()),
        );
        expect(session.requiresReopen, isTrue);
        await session.close();
      }
    },
  );

  test(
    'Quest outline edit publishes through full reopen CAS without a game root',
    () async {
      final outline = Revision3QuestOutlineFixture();
      final root = await _projectRoot(fixture, suffix: 'quest_outline');
      final store = _FakeRevision3Store(sealRegisteredHeads: true);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: outline.projectJson,
      );

      final published = await session.prepareAndPublishQuestOutlineEditV1(
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: outline.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: outline.moduleRevision,
        displayName: 'Find Homer safely',
        title: 'Find Homer safely',
        objectiveTitles: const [
          'Inspect the old gate',
          'Ask Asghan about Homer',
          'Report to Diego',
        ],
      );

      expect(store.questOutlinePrepareCalls, 1);
      expect(published.projectRevision, 8);
      expect(published.questRevision, 5);
      expect(published.moduleRevision, 6);
      expect(session.projectRevision, 8);
      expect(session.projectJson, published.projectJson);
      expect(
        await session.headFile.readAsString(),
        published.head.canonicalJson,
      );
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );

      await session.close();
      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectRevision, 8);
      expect(reopened.projectJson, published.projectJson);
      await reopened.close();
    },
  );

  test(
    'Quest outline semantic rejection retries but integrity failure poisons',
    () async {
      final outline = Revision3QuestOutlineFixture();
      final retryRoot = await _projectRoot(
        fixture,
        suffix: 'quest_outline_retry',
      );
      final retryStore = _FakeRevision3Store(sealRegisteredHeads: true);
      final retrySession = await ManagedRevision3AuthoringProjectSession.create(
        root: retryRoot,
        store: retryStore,
        projectJson: outline.projectJson,
      );
      retryStore.nextQuestOutlineError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_outline_edit_v1',
        code: 'AUTHORING_REVISION3_QUEST_OUTLINE_NO_CHANGES',
        message: 'fake semantic no-op',
      );
      await expectLater(
        retrySession.prepareAndPublishQuestOutlineEditV1(
          questId: revision3QuestOutlineQuestId,
          expectedQuestRevision: outline.questRevision,
          expectedModuleId: revision3QuestOutlineModuleId,
          expectedModuleRevision: outline.moduleRevision,
          displayName: 'Find Homer safely',
          title: 'Find Homer safely',
          objectiveTitles: const [
            'Inspect the old gate',
            'Ask Asghan about Homer',
            'Report to Diego',
          ],
        ),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            'AUTHORING_REVISION3_QUEST_OUTLINE_NO_CHANGES',
          ),
        ),
      );
      expect(retrySession.requiresReopen, isFalse);
      await retrySession.prepareAndPublishQuestOutlineEditV1(
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: outline.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: outline.moduleRevision,
        displayName: 'Find Homer safely',
        title: 'Find Homer safely',
        objectiveTitles: const [
          'Inspect the old gate',
          'Ask Asghan about Homer',
          'Report to Diego',
        ],
      );
      await retrySession.close();

      final poisonRoot = await _projectRoot(
        fixture,
        suffix: 'quest_outline_poison',
      );
      final poisonStore = _FakeRevision3Store(sealRegisteredHeads: true);
      final poisonSession =
          await ManagedRevision3AuthoringProjectSession.create(
            root: poisonRoot,
            store: poisonStore,
            projectJson: outline.projectJson,
          );
      poisonStore.nextQuestOutlineError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_outline_edit_v1',
        code: 'AUTHORING_REVISION3_QUEST_OUTLINE_PROJECT_INVALID',
        message: 'fake integrity failure',
      );
      await expectLater(
        poisonSession.prepareAndPublishQuestOutlineEditV1(
          questId: revision3QuestOutlineQuestId,
          expectedQuestRevision: outline.questRevision,
          expectedModuleId: revision3QuestOutlineModuleId,
          expectedModuleRevision: outline.moduleRevision,
          displayName: 'Find Homer safely',
          title: 'Find Homer safely',
          objectiveTitles: const [
            'Inspect the old gate',
            'Ask Asghan about Homer',
            'Report to Diego',
          ],
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(poisonSession.requiresReopen, isTrue);
      final calls = poisonStore.questOutlinePrepareCalls;
      await expectLater(
        poisonSession.prepareAndPublishQuestOutlineEditV1(
          questId: revision3QuestOutlineQuestId,
          expectedQuestRevision: outline.questRevision,
          expectedModuleId: revision3QuestOutlineModuleId,
          expectedModuleRevision: outline.moduleRevision,
          displayName: 'Another name',
          title: 'Another title',
          objectiveTitles: const [
            'Inspect the old gate',
            'Ask Asghan about Homer',
            'Report to Diego',
          ],
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(poisonStore.questOutlinePrepareCalls, calls);
      await poisonSession.close();
    },
  );

  test(
    'Quest transitions seed stays private and edit publishes through full reopen CAS',
    () async {
      final fixtureProject = Revision3QuestOutlineFixture();
      final root = await _projectRoot(fixture, suffix: 'quest_transitions');
      final store = _FakeRevision3Store(sealRegisteredHeads: true);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: fixtureProject.projectJson,
      );

      final seed = await session.readQuestTransitionsSeedV1(
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixtureProject.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: fixtureProject.moduleRevision,
      );
      expect(seed.projectRevision, fixtureProject.projectRevision);
      expect(seed.legacySynthetic, isTrue);
      expect(seed.objectives, hasLength(3));
      expect(store.questTransitionsPrepareCalls, 0);

      final published = await session.prepareAndPublishQuestTransitionsEditV1(
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixtureProject.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: fixtureProject.moduleRevision,
        expectedTransitionPlanSeal: seed.transitionPlanSeal,
        transitionPlan: seed.transitionPlan,
      );

      expect(store.questTransitionsPrepareCalls, 1);
      expect(store.questTransitionsRequests, hasLength(1));
      expect(published.projectRevision, fixtureProject.projectRevision + 1);
      expect(published.questRevision, fixtureProject.questRevision + 1);
      expect(published.moduleRevision, fixtureProject.moduleRevision + 1);
      expect(published.previousGeneratorVersion, 3);
      expect(published.upgradedFromLegacy, isTrue);
      expect(
        published.transitionPlanSeal.sha256,
        seed.transitionPlan.contentSeal.sha256,
      );
      expect(
        published.buildStatus,
        AuthoringRevision3QuestTransitionsBuildStatus.blocked,
      );
      expect(
        published.runtimeStatus,
        AuthoringRevision3QuestTransitionsRuntimeStatus.runtimeUnqualified,
      );
      expect(
        published.publicationStatus,
        AuthoringRevision3QuestTransitionsPublicationStatus.notSupported,
      );
      expect(session.projectJson, published.projectJson);
      expect(
        await session.headFile.readAsString(),
        published.head.canonicalJson,
      );
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );

      await session.close();
      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, published.projectJson);
      await reopened.close();
    },
  );

  test(
    'Quest transitions semantic rejection retries but integrity failure poisons',
    () async {
      final fixtureProject = Revision3QuestOutlineFixture();
      final retryRoot = await _projectRoot(
        fixture,
        suffix: 'quest_transitions_retry',
      );
      final retryStore = _FakeRevision3Store(sealRegisteredHeads: true);
      final retrySession = await ManagedRevision3AuthoringProjectSession.create(
        root: retryRoot,
        store: retryStore,
        projectJson: fixtureProject.projectJson,
      );
      final retrySeed = await retrySession.readQuestTransitionsSeedV1(
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixtureProject.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: fixtureProject.moduleRevision,
      );
      retryStore.nextQuestTransitionsError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_transitions_edit_v1',
        code: 'AUTHORING_REVISION3_QUEST_TRANSITIONS_TRANSITION_PLAN_CONFLICT',
        message: 'fake plan CAS conflict',
      );
      await expectLater(
        retrySession.prepareAndPublishQuestTransitionsEditV1(
          questId: revision3QuestOutlineQuestId,
          expectedQuestRevision: fixtureProject.questRevision,
          expectedModuleId: revision3QuestOutlineModuleId,
          expectedModuleRevision: fixtureProject.moduleRevision,
          expectedTransitionPlanSeal: retrySeed.transitionPlanSeal,
          transitionPlan: retrySeed.transitionPlan,
        ),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            'AUTHORING_REVISION3_QUEST_TRANSITIONS_TRANSITION_PLAN_CONFLICT',
          ),
        ),
      );
      expect(retrySession.requiresReopen, isFalse);
      await retrySession.prepareAndPublishQuestTransitionsEditV1(
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixtureProject.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: fixtureProject.moduleRevision,
        expectedTransitionPlanSeal: retrySeed.transitionPlanSeal,
        transitionPlan: retrySeed.transitionPlan,
      );
      await retrySession.close();

      final poisonRoot = await _projectRoot(
        fixture,
        suffix: 'quest_transitions_poison',
      );
      final poisonStore = _FakeRevision3Store(sealRegisteredHeads: true);
      final poisonSession =
          await ManagedRevision3AuthoringProjectSession.create(
            root: poisonRoot,
            store: poisonStore,
            projectJson: fixtureProject.projectJson,
          );
      final poisonSeed = await poisonSession.readQuestTransitionsSeedV1(
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixtureProject.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: fixtureProject.moduleRevision,
      );
      poisonStore.nextQuestTransitionsError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_transitions_edit_v1',
        code: 'AUTHORING_REVISION3_QUEST_TRANSITIONS_STORE_INVARIANT',
        message: 'fake transitions integrity failure',
      );
      await expectLater(
        poisonSession.prepareAndPublishQuestTransitionsEditV1(
          questId: revision3QuestOutlineQuestId,
          expectedQuestRevision: fixtureProject.questRevision,
          expectedModuleId: revision3QuestOutlineModuleId,
          expectedModuleRevision: fixtureProject.moduleRevision,
          expectedTransitionPlanSeal: poisonSeed.transitionPlanSeal,
          transitionPlan: poisonSeed.transitionPlan,
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(poisonSession.requiresReopen, isTrue);
      final calls = poisonStore.questTransitionsPrepareCalls;
      await expectLater(
        poisonSession.prepareAndPublishQuestTransitionsEditV1(
          questId: revision3QuestOutlineQuestId,
          expectedQuestRevision: fixtureProject.questRevision,
          expectedModuleId: revision3QuestOutlineModuleId,
          expectedModuleRevision: fixtureProject.moduleRevision,
          expectedTransitionPlanSeal: poisonSeed.transitionPlanSeal,
          transitionPlan: poisonSeed.transitionPlan,
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(poisonStore.questTransitionsPrepareCalls, calls);
      await poisonSession.close();
    },
  );

  test(
    'Quest context seed stays private and edit publishes through full reopen CAS',
    () async {
      final context = Revision3QuestOutlineFixture();
      final root = await _projectRoot(fixture, suffix: 'quest_context');
      final store = _FakeRevision3Store(sealRegisteredHeads: true);
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: context.projectJson,
      );

      final seed = await session.readQuestContextSeedV1(
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: context.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: context.moduleRevision,
        expectedParentRuntimeClass: 'UQuest_SwampCamp_SCChapter2',
        expectedGiverRuntimeUniqueName: 'OM_GRD_Asghan_263',
      );
      expect(seed.projectRevision, context.projectRevision);
      expect(seed.description, contains('missing worker'));
      expect(store.questContextPrepareCalls, 0);

      final published = await session.prepareAndPublishQuestContextEditV1(
        gameRoot: r'D:\Games\Gothic Remake',
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: context.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: context.moduleRevision,
        expectedStoryCatalogSeal: context.storyCatalogSeal,
        description: 'Find Homer and report back safely.',
        parentCatalogId: revision3QuestContextParentCatalogId,
        giverCatalogId: revision3QuestContextGiverCatalogId,
        expectedParentRuntimeClass: revision3QuestContextParentRuntimeClass,
        expectedParentCatalogLayer: 'base-game.quest-parent.v1',
        expectedParentAuthoringSelector: 'SwampCamp_SCChapter3',
        expectedParentSourceSeal: _contextSourceSeal(11, '1'),
        expectedGiverRuntimeUniqueName:
            revision3QuestContextGiverRuntimeUniqueName,
        expectedGiverCatalogLayer: 'base-game.npc.v1',
        expectedGiverAuthoringSelector:
            revision3QuestContextGiverRuntimeUniqueName,
        expectedGiverSourceSeal: _contextSourceSeal(12, '2'),
      );

      expect(store.questContextPrepareCalls, 1);
      expect(published.projectRevision, context.projectRevision + 1);
      expect(published.questRevision, context.questRevision + 1);
      expect(published.moduleRevision, context.moduleRevision + 1);
      expect(session.projectJson, published.projectJson);
      expect(
        await session.headFile.readAsString(),
        published.head.canonicalJson,
      );
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );

      await session.close();
      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, published.projectJson);
      await reopened.close();
    },
  );

  test(
    'Quest context external failures remain usable while head conflict poisons',
    () async {
      final context = Revision3QuestOutlineFixture();
      final retryRoot = await _projectRoot(
        fixture,
        suffix: 'quest_context_retry',
      );
      final retryStore = _FakeRevision3Store(sealRegisteredHeads: true);
      final retrySession = await ManagedRevision3AuthoringProjectSession.create(
        root: retryRoot,
        store: retryStore,
        projectJson: context.projectJson,
      );
      for (final code in <String>[
        'AUTHORING_REVISION3_QUEST_CONTEXT_CATALOG_CONFLICT',
        'AUTHORING_REVISION3_QUEST_CONTEXT_INPUT_MISSING',
        'AUTHORING_REVISION3_QUEST_CONTEXT_UNSUPPORTED_GENERATION',
        'AUTHORING_REVISION3_QUEST_CONTEXT_COLLISION_LIMIT',
        'AUTHORING_REVISION3_QUEST_CONTEXT_STORE_GAME_ALIAS',
      ]) {
        retryStore.nextQuestContextError = ModFfiException(
          command: 'authoring_store_prepare_revision3_quest_context_edit_v1',
          code: code,
          message: 'fake external context failure',
        );
        await expectLater(
          _publishQuestContext(retrySession, context),
          throwsA(
            isA<ModFfiException>().having((error) => error.code, 'code', code),
          ),
        );
        expect(retrySession.requiresReopen, isFalse, reason: code);
      }
      await _publishQuestContext(retrySession, context);
      await retrySession.close();

      final poisonRoot = await _projectRoot(
        fixture,
        suffix: 'quest_context_poison',
      );
      final poisonStore = _FakeRevision3Store(sealRegisteredHeads: true);
      final poisonSession =
          await ManagedRevision3AuthoringProjectSession.create(
            root: poisonRoot,
            store: poisonStore,
            projectJson: context.projectJson,
          );
      poisonStore.nextQuestContextError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_context_edit_v1',
        code: 'AUTHORING_REVISION3_QUEST_CONTEXT_HEAD_CONFLICT',
        message: 'fake exact head conflict',
      );
      await expectLater(
        _publishQuestContext(poisonSession, context),
        throwsA(isA<ManagedProjectHeadConflictException>()),
      );
      expect(poisonSession.requiresReopen, isTrue);
      final calls = poisonStore.questContextPrepareCalls;
      await expectLater(
        _publishQuestContext(poisonSession, context),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(poisonStore.questContextPrepareCalls, calls);
      await poisonSession.close();

      final provenanceRoot = await _projectRoot(
        fixture,
        suffix: 'quest_context_provenance_poison',
      );
      final provenanceStore = _FakeRevision3Store(sealRegisteredHeads: true)
        ..nextQuestContextProvenanceMismatch = 'selector';
      final provenanceSession =
          await ManagedRevision3AuthoringProjectSession.create(
            root: provenanceRoot,
            store: provenanceStore,
            projectJson: context.projectJson,
          );
      await expectLater(
        _publishQuestContext(provenanceSession, context),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(provenanceSession.requiresReopen, isTrue);
      expect(provenanceSession.projectRevision, context.projectRevision);
      await provenanceSession.close();
    },
  );

  test(
    'NPC local input and semantic collisions retry while integrity uncertainty poisons',
    () async {
      final retryRoot = await _projectRoot(fixture, suffix: 'npc_retry');
      final retryStore = _FakeRevision3Store();
      final retrySession = await ManagedRevision3AuthoringProjectSession.create(
        root: retryRoot,
        store: retryStore,
        projectJson: _projectJson(revision: 0, name: 'NPC retry'),
      );
      final exactRetryHead = await retrySession.headFile.readAsBytes();
      final beforeLocal = retryStore.npcPrepareCalls;
      await expectLater(
        retrySession.prepareAndPublishNpcDraftV1(
          gameRoot: r'D:\Games\Gothic Remake',
          npcId: '00000000000000000000000000000000',
          scriptModuleId: '00000000000000000000000000000082',
          displayName: 'Invalid local NPC',
          intent: _npcIntent(1),
        ),
        throwsFormatException,
      );
      expect(retryStore.npcPrepareCalls, beforeLocal);
      expect(retrySession.requiresReopen, isFalse);

      for (final retryableCode in <String>[
        'AUTHORING_REVISION3_NPC_COLLISION',
        'AUTHORING_REVISION3_NPC_INPUT_LIMIT',
        'AUTHORING_REVISION3_NPC_RECOVERY_REQUIRED',
      ]) {
        retryStore.nextNpcError = ModFfiException(
          command: 'authoring_store_prepare_revision3_npc_draft_v1',
          code: retryableCode,
          message: 'fake retryable NPC rejection',
        );
        await expectLater(
          retrySession.prepareAndPublishNpcDraftV1(
            gameRoot: r'D:\Games\Gothic Remake',
            npcId: '00000000000000000000000000000081',
            scriptModuleId: '00000000000000000000000000000082',
            displayName: 'Managed Guard',
            intent: _npcIntent(1),
          ),
          throwsA(
            isA<ModFfiException>().having(
              (error) => error.code,
              'code',
              retryableCode,
            ),
          ),
          reason: retryableCode,
        );
        expect(await retrySession.headFile.readAsBytes(), exactRetryHead);
        expect(retrySession.requiresReopen, isFalse);
      }
      await retrySession.prepareAndPublishNpcDraftV1(
        gameRoot: r'D:\Games\Gothic Remake',
        npcId: '00000000000000000000000000000081',
        scriptModuleId: '00000000000000000000000000000082',
        displayName: 'Managed Guard',
        intent: _npcIntent(1),
      );
      await retrySession.close();

      for (final errorCode in <String>[
        'AUTHORING_REVISION3_NPC_HEAD_CONFLICT',
        'AUTHORING_REVISION3_NPC_PROJECT_LIMIT',
        'AUTHORING_REVISION3_NPC_REQUEST_LIMIT',
        'AUTHORING_REVISION3_NPC_REVISION_LIMIT',
        'AUTHORING_REVISION3_NPC_STORE_SEAL_MISMATCH',
        ModFfiException.malformedNativeResponseCode,
        'AUTHORING_REVISION3_NPC_FUTURE_UNKNOWN',
      ]) {
        final root = await _projectRoot(
          fixture,
          suffix: errorCode.toLowerCase(),
        );
        final store = _FakeRevision3Store();
        final session = await ManagedRevision3AuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: _projectJson(revision: 0, name: errorCode),
        );
        final exactHead = await session.headFile.readAsBytes();
        store.nextNpcError = ModFfiException(
          command: 'authoring_store_prepare_revision3_npc_draft_v1',
          code: errorCode,
          message: 'fake NPC integrity failure',
        );
        await expectLater(
          session.prepareAndPublishNpcDraftV1(
            gameRoot: r'D:\Games\Gothic Remake',
            npcId: '00000000000000000000000000000081',
            scriptModuleId: '00000000000000000000000000000082',
            displayName: 'Managed Guard',
            intent: _npcIntent(1),
          ),
          throwsA(
            errorCode.endsWith('HEAD_CONFLICT')
                ? isA<ManagedProjectHeadConflictException>()
                : isA<ManagedProjectVerificationException>(),
          ),
          reason: errorCode,
        );
        expect(await session.headFile.readAsBytes(), exactHead);
        expect(session.requiresReopen, isTrue);
        final npcCalls = store.npcPrepareCalls;
        await expectLater(
          session.prepareAndPublishNpcDraftV1(
            gameRoot: r'D:\Games\Gothic Remake',
            npcId: '00000000000000000000000000000083',
            scriptModuleId: '00000000000000000000000000000084',
            displayName: 'Managed Guard 2',
            intent: _npcIntent(2),
          ),
          throwsA(isA<ManagedProjectVerificationException>()),
        );
        expect(store.npcPrepareCalls, npcCalls);
        await session.close();
      }
    },
  );

  test(
    'NPC candidate reopen mismatch stays unpublished and may be retried',
    () async {
      final root = await _projectRoot(fixture, suffix: 'npc_reopen');
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'NPC reopen');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      final exactHead = await session.headFile.readAsBytes();
      store.nextHeadOverride = store.register(
        _projectJson(revision: 70, name: 'Wrong NPC candidate reopen'),
      );

      await expectLater(
        session.prepareAndPublishNpcDraftV1(
          gameRoot: r'D:\Games\Gothic Remake',
          npcId: '00000000000000000000000000000081',
          scriptModuleId: '00000000000000000000000000000082',
          displayName: 'Managed Guard',
          intent: _npcIntent(1),
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(await session.headFile.readAsBytes(), exactHead);
      expect(session.projectJson, original);
      expect(session.requiresReopen, isFalse);

      final published = await session.prepareAndPublishNpcDraftV1(
        gameRoot: r'D:\Games\Gothic Remake',
        npcId: '00000000000000000000000000000081',
        scriptModuleId: '00000000000000000000000000000082',
        displayName: 'Managed Guard',
        intent: _npcIntent(1),
      );
      expect(session.head.canonicalJson, published.head.canonicalJson);
      await session.close();
    },
  );

  test(
    'head drift during native NPC prepare never clobbers the winner',
    () async {
      final root = await _projectRoot(fixture, suffix: 'npc_race');
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'NPC race'),
      );
      final externalProject = _projectJson(
        revision: 90,
        name: 'External NPC winner',
      );
      final externalHead = store.register(externalProject);
      store.afterNpcPrepare = (rootPath, _, _, _) => File(
        p.join(rootPath, 'gore-project.json'),
      ).writeAsString(externalHead.canonicalJson, flush: true);

      await expectLater(
        session.prepareAndPublishNpcDraftV1(
          gameRoot: r'D:\Games\Gothic Remake',
          npcId: '00000000000000000000000000000081',
          scriptModuleId: '00000000000000000000000000000082',
          displayName: 'Managed Guard',
          intent: _npcIntent(1),
        ),
        throwsA(isA<ManagedProjectHeadConflictException>()),
      );
      expect(await session.headFile.readAsString(), externalHead.canonicalJson);
      expect(session.projectRevision, 0);
      expect(session.requiresReopen, isTrue);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, externalProject);
      await reopened.close();
    },
  );

  test(
    'concurrent saves run in invocation order with present-head CAS',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Original'),
      );
      final firstEntered = Completer<void>();
      final releaseFirst = Completer<void>();
      store.afterPrepare = (_, _, projectJson) async {
        expect(projectJson, _projectJson(revision: 1, name: 'First'));
        firstEntered.complete();
        await releaseFirst.future;
      };

      final first = session.save(_projectJson(revision: 1, name: 'First'));
      await firstEntered.future;
      final second = session.save(_projectJson(revision: 2, name: 'Second'));
      await Future<void>.delayed(Duration.zero);
      expect(store.prepareCalls, 2); // create plus the blocked first save
      releaseFirst.complete();
      await Future.wait(<Future<void>>[first, second]);

      expect(session.projectJson, _projectJson(revision: 2, name: 'Second'));
      expect(store.expectedHeads, hasLength(3));
      expect(store.expectedHeads[0], isNull);
      expect(store.expectedHeads[1], isNotNull);
      expect(store.expectedHeads[2], isNotNull);
      expect(store.expectedHeads[2], isNot(store.expectedHeads[1]));
      await session.close();
    },
  );

  test(
    'verifyCurrentHead performs one full reopen without prepare or publish',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final project = _projectJson(revision: 0, name: 'Verified');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: project,
      );
      final exactHead = session.head.canonicalJson;
      final prepareCalls = store.prepareCalls;
      final openCalls = store.openVerifications.length;
      final headOpenCalls = store.headVerifications.length;
      final headBytes = await session.headFile.readAsBytes();

      await session.verifyCurrentHead();

      expect(store.prepareCalls, prepareCalls);
      expect(store.openVerifications.length, openCalls + 1);
      expect(store.openVerifications.last, AuthoringAssetVerification.full);
      expect(store.headVerifications.length, headOpenCalls);
      expect(session.head.canonicalJson, exactHead);
      expect(session.projectJson, project);
      expect(await session.headFile.readAsBytes(), headBytes);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'DataAsset prepare, list, and remove share guarded full-reopen publication',
    () async {
      final root = await _projectRoot(fixture, suffix: 'dataasset_roundtrip');
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'DataAsset roundtrip'),
      );
      final initialPrepareCalls = store.prepareCalls;
      final initialHeadOpenCalls = store.headVerifications.length;
      final initialHead = session.head.canonicalJson;

      expect(await session.listDataAssetStagesV1(), isEmpty);
      expect(session.head.canonicalJson, initialHead);
      final staged = await session.prepareAndPublishDataAssetStageV1(
        patchReceiptPath: r'C:\Receipts\managed-patch.v2.json',
      );

      expect(staged.projectId, session.projectId);
      expect(staged.projectRevision, 1);
      expect(staged.head.canonicalJson, session.head.canonicalJson);
      expect(staged.stage.targetPath, revision3DataAssetTargetPath);
      expect(staged.stage.basisProjectRevision, 0);
      expect(staged.stage.stagedProjectRevision, 1);
      expect(store.dataAssetPrepareCalls, 1);
      expect(store.prepareCalls, initialPrepareCalls);
      expect(
        store.headVerifications.length,
        greaterThanOrEqualTo(initialHeadOpenCalls + 2),
      );
      expect(
        store.headVerifications
            .skip(initialHeadOpenCalls)
            .every((value) => value == AuthoringAssetVerification.full),
        isTrue,
      );

      final exactPublishedHead = await session.headFile.readAsBytes();
      final listed = await session.listDataAssetStagesV1();
      expect(listed, hasLength(1));
      expect(
        listed.single.manifestAsset.sha256,
        staged.stage.manifestAsset.sha256,
      );
      expect(await session.headFile.readAsBytes(), exactPublishedHead);

      final removed = await session.prepareAndPublishRemoveDataAssetStageV1(
        targetPath: revision3DataAssetTargetPath.toLowerCase(),
      );
      expect(removed.projectRevision, 2);
      expect(removed.removed.targetPath, revision3DataAssetTargetPath);
      expect(session.projectRevision, 2);
      expect(await session.listDataAssetStagesV1(), isEmpty);
      expect(store.dataAssetRemoveCalls, 1);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'DataAsset head drift during native prepare never clobbers the winner',
    () async {
      final root = await _projectRoot(fixture, suffix: 'dataasset_race');
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'DataAsset race');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      final externalProject = _projectJson(
        revision: 90,
        name: 'External DataAsset winner',
      );
      final externalHead = store.register(externalProject);
      store.afterDataAssetPrepare = (rootPath, _, _) => File(
        p.join(rootPath, 'gore-project.json'),
      ).writeAsString(externalHead.canonicalJson, flush: true);

      await expectLater(
        session.prepareAndPublishDataAssetStageV1(
          patchReceiptPath: r'C:\Receipts\race.v2.json',
        ),
        throwsA(isA<ManagedProjectHeadConflictException>()),
      );

      expect(await session.headFile.readAsString(), externalHead.canonicalJson);
      expect(session.projectJson, original);
      expect(session.requiresReopen, isTrue);
      await session.close();
    },
  );

  test(
    'semantic DataAsset edit publishes through guarded full reopen and lists exact stage',
    () async {
      final root = await _projectRoot(fixture, suffix: 'dataasset_edit');
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'DataAsset value edit'),
      );
      final headOpens = store.headVerifications.length;
      final intent = _semanticDataAssetIntent();

      final published = await session.prepareAndPublishDataAssetEditV1(
        intent: intent,
      );

      expect(store.dataAssetEditPrepareCalls, 1);
      expect(store.dataAssetEditIntents.single, same(intent));
      expect(published.projectRevision, 1);
      expect(published.stage.targetPath, revision3DataAssetTargetPath);
      expect(published.stage.selectorKind, 'int32');
      expect(published.stage.replacementByteLength, 4);
      expect(
        store.headVerifications.skip(headOpens),
        everyElement(AuthoringAssetVerification.full),
      );
      final exactHeadBytes = await session.headFile.readAsBytes();
      final listed = await session.listDataAssetStagesV1();
      expect(listed, hasLength(1));
      expect(listed.single.targetPath, intent.expectedTargetPath);
      expect(await session.headFile.readAsBytes(), exactHeadBytes);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'semantic DataAsset edit head drift preserves winner and requires reopen',
    () async {
      final root = await _projectRoot(fixture, suffix: 'dataasset_edit_race');
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'DataAsset edit race');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      final external = _projectJson(
        revision: 77,
        name: 'External semantic edit winner',
      );
      final externalHead = store.register(external);
      store.afterDataAssetPrepare = (rootPath, _, _) => File(
        p.join(rootPath, 'gore-project.json'),
      ).writeAsString(externalHead.canonicalJson, flush: true);

      await expectLater(
        session.prepareAndPublishDataAssetEditV1(
          intent: _semanticDataAssetIntent(),
        ),
        throwsA(isA<ManagedProjectHeadConflictException>()),
      );

      expect(await session.headFile.readAsString(), externalHead.canonicalJson);
      expect(session.projectJson, original);
      expect(session.requiresReopen, isTrue);
      await session.close();
    },
  );

  test(
    'semantic DataAsset edit keeps input failures retryable and poisons binding drift',
    () async {
      final retryRoot = await _projectRoot(
        fixture,
        suffix: 'dataasset_edit_retry',
      );
      final retryStore = _FakeRevision3Store();
      final retrySession = await ManagedRevision3AuthoringProjectSession.create(
        root: retryRoot,
        store: retryStore,
        projectJson: _projectJson(revision: 0, name: 'DataAsset edit retry'),
      );
      retryStore.nextDataAssetError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_dataasset_edit_v1',
        code: 'AUTHORING_REVISION3_DATAASSET_EDIT_INVALID',
        message: 'fake semantic input rejection',
      );
      await expectLater(
        retrySession.prepareAndPublishDataAssetEditV1(
          intent: _semanticDataAssetIntent(),
        ),
        throwsA(isA<ModFfiException>()),
      );
      expect(retrySession.requiresReopen, isFalse);
      expect(retrySession.projectRevision, 0);
      await retrySession.close();

      final poisonRoot = await _projectRoot(
        fixture,
        suffix: 'dataasset_edit_binding_poison',
      );
      final poisonStore = _FakeRevision3Store();
      final poisonSession =
          await ManagedRevision3AuthoringProjectSession.create(
            root: poisonRoot,
            store: poisonStore,
            projectJson: _projectJson(
              revision: 0,
              name: 'DataAsset edit binding poison',
            ),
          );
      final exactHead = await poisonSession.headFile.readAsBytes();
      poisonStore.nextDataAssetResponseMismatch = 'intent-binding';
      await expectLater(
        poisonSession.prepareAndPublishDataAssetEditV1(
          intent: _semanticDataAssetIntent(),
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(await poisonSession.headFile.readAsBytes(), exactHead);
      expect(poisonSession.projectRevision, 0);
      expect(poisonSession.requiresReopen, isTrue);
      await poisonSession.close();
    },
  );

  test(
    'DataAsset response limits and local preflight are retryable; integrity failures poison',
    () async {
      final retryRoot = await _projectRoot(fixture, suffix: 'dataasset_retry');
      final retryStore = _FakeRevision3Store();
      final retrySession = await ManagedRevision3AuthoringProjectSession.create(
        root: retryRoot,
        store: retryStore,
        projectJson: _projectJson(revision: 0, name: 'DataAsset retry'),
      );
      retryStore.nextDataAssetError = const ModFfiException(
        command: 'authoring_store_list_revision3_dataasset_stages_v1',
        code: 'AUTHORING_REVISION3_DATAASSET_RESPONSE_LIMIT',
        message: 'fake bounded list response limit',
      );
      await expectLater(
        retrySession.listDataAssetStagesV1(),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            'AUTHORING_REVISION3_DATAASSET_RESPONSE_LIMIT',
          ),
        ),
      );
      expect(retrySession.requiresReopen, isFalse);

      retryStore.nextDataAssetError = ArgumentError(
        'fake local path preflight',
      );
      await expectLater(
        retrySession.prepareAndPublishDataAssetStageV1(
          patchReceiptPath: 'bad input',
        ),
        throwsArgumentError,
      );
      expect(retrySession.requiresReopen, isFalse);
      expect(await retrySession.listDataAssetStagesV1(), isEmpty);
      await retrySession.close();

      final poisonRoot = await _projectRoot(
        fixture,
        suffix: 'dataasset_poison',
      );
      final poisonStore = _FakeRevision3Store();
      final poisonSession =
          await ManagedRevision3AuthoringProjectSession.create(
            root: poisonRoot,
            store: poisonStore,
            projectJson: _projectJson(revision: 0, name: 'DataAsset poison'),
          );
      poisonStore.nextDataAssetError = const ModFfiException(
        command: 'authoring_store_list_revision3_dataasset_stages_v1',
        code: 'AUTHORING_REVISION3_DATAASSET_STORE_SEAL_MISMATCH',
        message: 'fake DataAsset integrity failure',
      );
      await expectLater(
        poisonSession.listDataAssetStagesV1(),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(poisonSession.requiresReopen, isTrue);
      await poisonSession.close();
    },
  );

  test(
    'DataAsset malformed candidate response poisons without publication',
    () async {
      final root = await _projectRoot(fixture, suffix: 'dataasset_mismatch');
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'DataAsset mismatch'),
      );
      final exactHead = await session.headFile.readAsBytes();
      store.nextDataAssetResponseMismatch = 'revision';

      await expectLater(
        session.prepareAndPublishDataAssetStageV1(
          patchReceiptPath: r'C:\Receipts\mismatch.v2.json',
        ),
        throwsA(isA<ManagedProjectVerificationException>()),
      );

      expect(await session.headFile.readAsBytes(), exactHead);
      expect(session.projectRevision, 0);
      expect(session.requiresReopen, isTrue);
      await session.close();
    },
  );

  test(
    'content read stays exact-head, serialized, and publication-free',
    () async {
      final root = await _projectRoot(fixture, suffix: 'content_exact');
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'Content exact');
      final saved = _projectJson(revision: 1, name: 'After content');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      final prepareCalls = store.prepareCalls;
      final openCalls = store.openVerifications.length;
      final headOpenCalls = store.headVerifications.length;
      final exactHead = session.head.canonicalJson;
      final headBytes = await session.headFile.readAsBytes();
      final readEntered = Completer<void>();
      final releaseRead = Completer<void>();
      final savePrepareEntered = Completer<void>();
      final releaseSavePrepare = Completer<void>();
      store.afterContentRead = (_, _, _) async {
        readEntered.complete();
        await releaseRead.future;
      };
      store.afterPrepare = (_, _, _) async {
        savePrepareEntered.complete();
        await releaseSavePrepare.future;
      };

      final reading = session.readContentIndex();
      await readEntered.future;
      final saving = session.save(saved);
      await Future<void>.delayed(Duration.zero);
      expect(store.prepareCalls, prepareCalls);
      expect(store.openVerifications.length, openCalls);
      expect(store.headVerifications.length, headOpenCalls);

      releaseRead.complete();
      final index = await reading;
      await savePrepareEntered.future;
      expect(await session.headFile.readAsBytes(), orderedEquals(headBytes));
      expect(session.projectJson, original);
      releaseSavePrepare.complete();
      await saving;

      expect(index.projectId, '00000000000000000000000000000003');
      expect(index.projectRevision, 0);
      expect(index.projectName, 'Content exact');
      expect(store.contentReadCalls, 1);
      expect(store.contentExpectedHeads, <String>[exactHead]);
      expect(store.prepareCalls, prepareCalls + 1);
      expect(session.projectJson, saved);
      expect(session.projectRevision, 1);
      expect(
        await session.headFile.readAsBytes(),
        isNot(orderedEquals(headBytes)),
      );
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test('content read head drift poisons without publishing', () async {
    final root = await _projectRoot(fixture, suffix: 'content_drift');
    final store = _FakeRevision3Store();
    final original = _projectJson(revision: 0, name: 'Content drift');
    final session = await ManagedRevision3AuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: original,
    );
    final prepareCalls = store.prepareCalls;
    final external = store.register(
      _projectJson(revision: 91, name: 'External content winner'),
    );
    store.afterContentRead = (rootPath, _, _) => File(
      p.join(rootPath, 'gore-project.json'),
    ).writeAsString(external.canonicalJson, flush: true);

    await expectLater(
      session.readContentIndex(),
      throwsA(isA<ManagedProjectHeadConflictException>()),
    );

    expect(await session.headFile.readAsString(), external.canonicalJson);
    expect(store.prepareCalls, prepareCalls);
    expect(session.projectJson, original);
    expect(session.requiresReopen, isTrue);
    await expectLater(
      session.readContentIndex(),
      throwsA(isA<ManagedProjectVerificationException>()),
    );
    expect(store.contentReadCalls, 1);
    await session.close();
  });

  test(
    'content capacity rejection is retryable while integrity failures poison',
    () async {
      final retryRoot = await _projectRoot(fixture, suffix: 'content_retry');
      final retryStore = _FakeRevision3Store();
      final retrySession = await ManagedRevision3AuthoringProjectSession.create(
        root: retryRoot,
        store: retryStore,
        projectJson: _projectJson(revision: 0, name: 'Content retry'),
      );
      retryStore.nextContentError = const ModFfiException(
        command: 'authoring_store_read_revision3_content_index_v1',
        code: 'AUTHORING_REVISION3_CONTENT_RESPONSE_LIMIT',
        message: 'fake bounded content limit',
      );
      await expectLater(
        retrySession.readContentIndex(),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            'AUTHORING_REVISION3_CONTENT_RESPONSE_LIMIT',
          ),
        ),
      );
      expect(retrySession.requiresReopen, isFalse);
      expect((await retrySession.readContentIndex()).projectRevision, 0);
      await retrySession.close();

      final poisonRoot = await _projectRoot(fixture, suffix: 'content_poison');
      final poisonStore = _FakeRevision3Store();
      final poisonSession =
          await ManagedRevision3AuthoringProjectSession.create(
            root: poisonRoot,
            store: poisonStore,
            projectJson: _projectJson(revision: 0, name: 'Content poison'),
          );
      poisonStore.nextContentError = const ModFfiException(
        command: 'authoring_store_read_revision3_content_index_v1',
        code: 'AUTHORING_REVISION3_CONTENT_STORE_SEAL_MISMATCH',
        message: 'fake content integrity failure',
      );
      await expectLater(
        poisonSession.readContentIndex(),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(poisonSession.requiresReopen, isTrue);
      await poisonSession.close();
    },
  );

  test('content response identity mismatch fails closed', () async {
    final root = await _projectRoot(fixture, suffix: 'content_identity');
    final store = _FakeRevision3Store();
    final session = await ManagedRevision3AuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: _projectJson(revision: 0, name: 'Content identity'),
    );
    final exactHead = await session.headFile.readAsBytes();
    store.nextContentResponseMismatch = 'project-id';

    await expectLater(
      session.readContentIndex(),
      throwsA(isA<ManagedProjectVerificationException>()),
    );

    expect(await session.headFile.readAsBytes(), exactHead);
    expect(session.requiresReopen, isTrue);
    await session.close();
  });

  test(
    'Quest source inspection forwards the exact read-only basis without publishing',
    () async {
      final root = await _projectRoot(
        fixture,
        suffix: 'quest_inspection_exact',
      );
      final store = _FakeRevision3Store();
      final projectJson = _projectJson(
        revision: 12,
        name: 'Quest inspection exact',
      );
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: projectJson,
      );
      const gameRoot = r'D:\Games\Gothic Remake';
      const questId = '00000000000000000000000000000071';
      final exactHead = session.head.canonicalJson;
      final exactHeadBytes = await session.headFile.readAsBytes();
      final prepareCalls = store.prepareCalls;
      final questPrepareCalls = store.questPrepareCalls;

      final result = await session.inspectQuestSourceV1(
        gameRoot: gameRoot,
        questId: questId,
      );

      expect(result.head.canonicalJson, exactHead);
      expect(result.projectId, session.projectId);
      expect(result.projectRevision, 12);
      expect(result.questId, questId);
      expect(result.generatedSource, contains('UQuest_GoreInspection'));
      expect(store.questInspectionCalls, 1);
      expect(store.questInspectionRoots, <String>[root.path]);
      expect(store.questInspectionGameRoots, <String>[gameRoot]);
      expect(store.questInspectionExpectedHeads, <String>[exactHead]);
      expect(store.questInspectionQuestIds, <String>[questId]);
      expect(store.prepareCalls, prepareCalls);
      expect(store.questPrepareCalls, questPrepareCalls);
      expect(
        await session.headFile.readAsBytes(),
        orderedEquals(exactHeadBytes),
      );
      expect(session.projectJson, projectJson);
      expect(session.projectRevision, 12);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'Quest source inspection rejects every response basis mismatch fail-closed',
    () async {
      for (final mismatch in <String>[
        'head',
        'project-id',
        'revision',
        'quest',
        'project-seal',
      ]) {
        final root = await _projectRoot(
          fixture,
          suffix: 'quest_inspection_$mismatch',
        );
        final store = _FakeRevision3Store();
        final session = await ManagedRevision3AuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: _projectJson(
            revision: 13,
            name: 'Quest inspection $mismatch',
          ),
        );
        final exactHeadBytes = await session.headFile.readAsBytes();
        final prepareCalls = store.prepareCalls;
        store.nextQuestInspectionResponseMismatch = mismatch;

        await expectLater(
          session.inspectQuestSourceV1(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000071',
          ),
          throwsA(isA<ManagedProjectVerificationException>()),
          reason: mismatch,
        );

        expect(store.questInspectionCalls, 1, reason: mismatch);
        expect(store.prepareCalls, prepareCalls, reason: mismatch);
        expect(
          await session.headFile.readAsBytes(),
          orderedEquals(exactHeadBytes),
          reason: mismatch,
        );
        expect(session.projectRevision, 13, reason: mismatch);
        expect(session.requiresReopen, isTrue, reason: mismatch);
        await session.close();
      }
    },
  );

  test('Quest source inspection detects a post-read exact-head race', () async {
    final root = await _projectRoot(
      fixture,
      suffix: 'quest_inspection_head_race',
    );
    final store = _FakeRevision3Store();
    final session = await ManagedRevision3AuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: _projectJson(
        revision: 14,
        name: 'Quest inspection head race',
      ),
    );
    final prepareCalls = store.prepareCalls;
    final external = store.register(
      _projectJson(revision: 91, name: 'External inspection winner'),
    );
    store.afterQuestInspection = (rootPath, _, _) => File(
      p.join(rootPath, 'gore-project.json'),
    ).writeAsString(external.canonicalJson, flush: true);

    await expectLater(
      session.inspectQuestSourceV1(
        gameRoot: r'D:\Games\Gothic Remake',
        questId: '00000000000000000000000000000071',
      ),
      throwsA(isA<ManagedProjectHeadConflictException>()),
    );

    expect(await session.headFile.readAsString(), external.canonicalJson);
    expect(store.prepareCalls, prepareCalls);
    expect(store.questInspectionCalls, 1);
    expect(session.projectRevision, 14);
    expect(session.requiresReopen, isTrue);
    await expectLater(
      session.inspectQuestSourceV1(
        gameRoot: r'D:\Games\Gothic Remake',
        questId: '00000000000000000000000000000071',
      ),
      throwsA(isA<ManagedProjectVerificationException>()),
    );
    expect(store.questInspectionCalls, 1);
    await session.close();
  });

  test(
    'Quest inspection domain errors retry while malformed, integrity, and head conflict poison',
    () async {
      final retryRoot = await _projectRoot(
        fixture,
        suffix: 'quest_inspection_retry',
      );
      final retryStore = _FakeRevision3Store();
      final retrySession = await ManagedRevision3AuthoringProjectSession.create(
        root: retryRoot,
        store: retryStore,
        projectJson: _projectJson(revision: 15, name: 'Quest inspection retry'),
      );
      final retryableCodes = <String>[
        'AUTHORING_REVISION3_QUEST_INSPECTION_COLLISION_LIMIT',
        'AUTHORING_REVISION3_QUEST_INSPECTION_FAILED',
        'AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_CHANGED',
        'AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_LIMIT',
        'AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_MISSING',
        'AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_UNAVAILABLE',
        'AUTHORING_REVISION3_QUEST_INSPECTION_INPUT_UNSAFE',
        'AUTHORING_REVISION3_QUEST_INSPECTION_INVENTORY_FAILED',
        'AUTHORING_REVISION3_QUEST_INSPECTION_PROJECT_INVALID',
        'AUTHORING_REVISION3_QUEST_INSPECTION_PROJECT_TARGET_MISMATCH',
        'AUTHORING_REVISION3_QUEST_INSPECTION_QUEST_INVALID',
        'AUTHORING_REVISION3_QUEST_INSPECTION_RECOVERY_REQUIRED',
        'AUTHORING_REVISION3_QUEST_INSPECTION_REQUEST_INVALID',
        'AUTHORING_REVISION3_QUEST_INSPECTION_RESPONSE_LIMIT',
        'AUTHORING_REVISION3_QUEST_INSPECTION_UNSUPPORTED_GENERATION',
      ];
      for (final code in retryableCodes) {
        retryStore.nextQuestInspectionError = ModFfiException(
          command: 'authoring_store_inspect_revision3_quest_source_v1',
          code: code,
          message: 'fake retryable Quest inspection domain error',
        );
        await expectLater(
          retrySession.inspectQuestSourceV1(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000071',
          ),
          throwsA(
            isA<ModFfiException>().having((error) => error.code, 'code', code),
          ),
          reason: code,
        );
        expect(retrySession.requiresReopen, isFalse, reason: code);
      }
      expect(
        (await retrySession.inspectQuestSourceV1(
          gameRoot: r'D:\Games\Gothic Remake',
          questId: '00000000000000000000000000000071',
        )).projectRevision,
        15,
      );
      expect(retryStore.questInspectionCalls, retryableCodes.length + 1);
      await retrySession.close();

      final poisonCodes = <String>[
        ModFfiException.malformedNativeResponseCode,
        'AUTHORING_REVISION3_QUEST_INSPECTION_STORE_SEAL_MISMATCH',
        'AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_CONFLICT',
      ];
      var ordinal = 0;
      for (final code in poisonCodes) {
        ordinal++;
        final poisonRoot = await _projectRoot(
          fixture,
          suffix: 'quest_inspection_poison_$ordinal',
        );
        final poisonStore = _FakeRevision3Store();
        final poisonSession =
            await ManagedRevision3AuthoringProjectSession.create(
              root: poisonRoot,
              store: poisonStore,
              projectJson: _projectJson(
                revision: 15 + ordinal,
                name: 'Quest inspection poison $ordinal',
              ),
            );
        poisonStore.nextQuestInspectionError = ModFfiException(
          command: 'authoring_store_inspect_revision3_quest_source_v1',
          code: code,
          message: 'fake fail-closed Quest inspection error',
        );

        await expectLater(
          poisonSession.inspectQuestSourceV1(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000071',
          ),
          throwsA(
            code == 'AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_CONFLICT'
                ? isA<ManagedProjectHeadConflictException>()
                : isA<ManagedProjectVerificationException>(),
          ),
          reason: code,
        );
        expect(poisonSession.requiresReopen, isTrue, reason: code);
        await expectLater(
          poisonSession.inspectQuestSourceV1(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000071',
          ),
          throwsA(isA<ManagedProjectVerificationException>()),
          reason: code,
        );
        expect(poisonStore.questInspectionCalls, 1, reason: code);
        await poisonSession.close();
      }
    },
  );

  test(
    'verifyCurrentHead drift or reopen mismatch poisons the session',
    () async {
      for (final mode in <String>['head-drift', 'reopen-mismatch']) {
        final root = await _projectRoot(fixture, suffix: mode);
        final store = _FakeRevision3Store();
        final original = _projectJson(revision: 0, name: 'Original $mode');
        final session = await ManagedRevision3AuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: original,
        );
        final prepareCalls = store.prepareCalls;
        if (mode == 'head-drift') {
          final external = store.register(
            _projectJson(revision: 91, name: 'External'),
          );
          await session.headFile.writeAsString(
            external.canonicalJson,
            flush: true,
          );
        } else {
          store.nextOpenProjectOverride = _projectJson(
            revision: 92,
            name: 'Wrong reopen',
          );
        }

        await expectLater(
          session.verifyCurrentHead(),
          throwsA(isA<ManagedProjectSessionException>()),
          reason: mode,
        );
        expect(store.prepareCalls, prepareCalls, reason: mode);
        expect(session.projectJson, original, reason: mode);
        expect(session.requiresReopen, isTrue, reason: mode);
        await expectLater(
          session.verifyCurrentHead(),
          throwsA(isA<ManagedProjectVerificationException>()),
          reason: mode,
        );
        await session.close();
      }
    },
  );

  test(
    'derive rejection and callback throw prepare and publish nothing',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'Original');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      final prepares = store.prepareCalls;
      final exactHead = await session.headFile.readAsBytes();

      final rejection = await session.deriveAndSave<int>((latest) {
        expect(latest, original);
        return const ManagedProjectDerivedRejection<int>(41);
      });
      expect(rejection, 41);
      expect(store.prepareCalls, prepares);
      expect(await session.headFile.readAsBytes(), exactHead);

      await expectLater(
        session.deriveAndSave<void>((_) => throw StateError('derive failed')),
        throwsA(isA<StateError>()),
      );
      expect(store.prepareCalls, prepares);
      expect(await session.headFile.readAsBytes(), exactHead);
      expect(session.requiresReopen, isFalse);
      await session.close();
    },
  );

  test(
    'an external race never clobbers and poisons edits until reopen',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Original'),
      );
      final externalProject = _projectJson(revision: 90, name: 'External');
      final externalHead = store.register(externalProject);
      store.afterPrepare = (rootPath, _, _) => File(
        p.join(rootPath, 'gore-project.json'),
      ).writeAsString(externalHead.canonicalJson, flush: true);

      await expectLater(
        session.save(_projectJson(revision: 1, name: 'Must not win')),
        throwsA(isA<ManagedProjectHeadConflictException>()),
      );
      expect(await session.headFile.readAsString(), externalHead.canonicalJson);
      expect(session.requiresReopen, isTrue);
      final prepares = store.prepareCalls;
      await expectLater(
        session.save(_projectJson(revision: 2, name: 'Still rejected')),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(store.prepareCalls, prepares);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, externalProject);
      expect(reopened.requiresReopen, isFalse);
      await reopened.close();
    },
  );

  test(
    'interrupted publication is repaired by a full verified reopen',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      var armed = false;
      final replacement = AtomicByteReplacement(
        operationIdFactory: () => '73000000000000000000000000000001',
        onPhase: (phase) {
          if (armed && phase == AtomicSwapPhase.tempPromoted) {
            throw const AtomicSwapException('injected publication failure');
          }
        },
      );
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Original'),
        replacement: replacement,
      );
      armed = true;
      final saved = _projectJson(revision: 1, name: 'Recovered');

      await expectLater(
        session.save(saved),
        throwsA(isA<AtomicSwapException>()),
      );
      expect(session.requiresReopen, isTrue);
      final journal = File(
        AtomicByteReplacement.journalPathFor(session.headFile),
      );
      expect(await journal.exists(), isTrue);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, saved);
      expect(await journal.exists(), isFalse);
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      await reopened.close();
    },
  );

  test(
    'interrupted Quest publication repairs the verified prepared candidate',
    () async {
      final root = await _projectRoot(fixture, suffix: 'quest_repair');
      final store = _FakeRevision3Store();
      var armed = false;
      final replacement = AtomicByteReplacement(
        operationIdFactory: () => '75000000000000000000000000000001',
        onPhase: (phase) {
          if (armed && phase == AtomicSwapPhase.tempPromoted) {
            throw const AtomicSwapException(
              'injected Quest publication failure',
            );
          }
        },
      );
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Quest repair'),
        replacement: replacement,
      );
      armed = true;

      await expectLater(
        session.prepareAndPublishQuestDraftV3(
          gameRoot: r'D:\Games\Gothic Remake',
          questId: '00000000000000000000000000000071',
          scriptModuleId: '00000000000000000000000000000072',
          displayName: 'Managed Quest 1',
          intent: _questIntent(1),
        ),
        throwsA(isA<AtomicSwapException>()),
      );
      expect(session.projectRevision, 0);
      expect(session.requiresReopen, isTrue);
      final journal = File(
        AtomicByteReplacement.journalPathFor(session.headFile),
      );
      expect(await journal.exists(), isTrue);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectRevision, 1);
      expect(await journal.exists(), isFalse);
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      await reopened.close();
    },
  );

  test(
    'raw filesystem publication failure poisons and repairs on reopen',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      var armed = false;
      final replacement = AtomicByteReplacement(
        operationIdFactory: () => '74000000000000000000000000000001',
        onPhase: (phase) {
          if (armed && phase == AtomicSwapPhase.targetBackedUp) {
            throw FileSystemException('injected raw publication failure');
          }
        },
      );
      final original = _projectJson(revision: 0, name: 'Original');
      final saved = _projectJson(revision: 1, name: 'Recovered raw failure');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
        replacement: replacement,
      );
      final originalHead = session.head.canonicalJson;
      armed = true;

      await expectLater(
        session.save(saved),
        throwsA(isA<FileSystemException>()),
      );
      expect(session.requiresReopen, isTrue);
      expect(session.projectJson, original);
      expect(session.head.canonicalJson, originalHead);
      expect(await session.headFile.exists(), isFalse);
      final journal = File(
        AtomicByteReplacement.journalPathFor(session.headFile),
      );
      expect(await journal.exists(), isTrue);

      final preparesAfterFailure = store.prepareCalls;
      await expectLater(
        session.save(_projectJson(revision: 2, name: 'Must not prepare')),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(store.prepareCalls, preparesAfterFailure);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, saved);
      expect(reopened.projectRevision, 1);
      expect(reopened.requiresReopen, isFalse);
      expect(
        await reopened.headFile.readAsString(),
        reopened.head.canonicalJson,
      );
      expect(await journal.exists(), isFalse);
      expect(
        store.headVerifications,
        everyElement(AuthoringAssetVerification.full),
      );
      await reopened.close();
    },
  );

  test(
    'post-publication mismatch poisons session but reopen recovers',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final original = _projectJson(revision: 0, name: 'Original');
      final saved = _projectJson(revision: 1, name: 'Saved');
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: original,
      );
      store.nextOpenProjectOverride = _projectJson(
        revision: 99,
        name: 'Mismatch',
      );

      await expectLater(
        session.save(saved),
        throwsA(isA<ManagedProjectVerificationException>()),
      );
      expect(session.projectJson, original);
      expect(session.requiresReopen, isTrue);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, saved);
      await reopened.close();
    },
  );

  test(
    'prepared candidate head and project mismatches never publish',
    () async {
      for (final mismatch in <String>['head', 'project']) {
        final root = await _projectRoot(fixture, suffix: mismatch);
        final store = _FakeRevision3Store();
        final original = _projectJson(revision: 0, name: 'Original $mismatch');
        final candidate = _projectJson(
          revision: 1,
          name: 'Candidate $mismatch',
        );
        final session = await ManagedRevision3AuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: original,
        );
        final exactPublishedHead = await session.headFile.readAsBytes();
        if (mismatch == 'head') {
          store.nextHeadOverride = store.register(
            _projectJson(revision: 88, name: 'Wrong head'),
          );
        } else {
          store.nextHeadProjectOverride = _projectJson(
            revision: 89,
            name: 'Wrong project',
          );
        }

        await expectLater(
          session.save(candidate),
          throwsA(isA<ManagedProjectVerificationException>()),
          reason: mismatch,
        );
        expect(await session.headFile.readAsBytes(), exactPublishedHead);
        expect(session.projectJson, original);
        expect(session.requiresReopen, isFalse);

        await session.save(candidate);
        expect(session.projectJson, candidate);
        await session.close();
      }
    },
  );

  test(
    'Quest basis, response, and full-reopen mismatches never publish',
    () async {
      for (final mismatch in <String>[
        'basis-head',
        'candidate-project',
        'revision',
        'display-name',
        'candidate-reopen',
      ]) {
        final root = await _projectRoot(fixture, suffix: 'quest_$mismatch');
        final store = _FakeRevision3Store();
        final original = _projectJson(revision: 0, name: 'Quest $mismatch');
        final session = await ManagedRevision3AuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: original,
        );
        final exactHeadBytes = await session.headFile.readAsBytes();
        final genericPrepares = store.prepareCalls;
        if (mismatch == 'candidate-reopen') {
          store.nextHeadOverride = store.register(
            _projectJson(revision: 70, name: 'Wrong candidate reopen'),
          );
        } else {
          store.nextQuestResponseMismatch = mismatch;
        }

        await expectLater(
          session.prepareAndPublishQuestDraftV3(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000071',
            scriptModuleId: '00000000000000000000000000000072',
            displayName: 'Managed Quest 1',
            intent: _questIntent(1),
          ),
          throwsA(isA<ManagedProjectVerificationException>()),
          reason: mismatch,
        );
        expect(
          await session.headFile.readAsBytes(),
          exactHeadBytes,
          reason: mismatch,
        );
        expect(session.projectJson, original, reason: mismatch);
        final poisoned = mismatch != 'candidate-reopen';
        expect(session.requiresReopen, poisoned, reason: mismatch);
        expect(store.prepareCalls, genericPrepares, reason: mismatch);

        if (poisoned) {
          await expectLater(
            session.prepareAndPublishQuestDraftV3(
              gameRoot: r'D:\Games\Gothic Remake',
              questId: '00000000000000000000000000000071',
              scriptModuleId: '00000000000000000000000000000072',
              displayName: 'Managed Quest 1',
              intent: _questIntent(1),
            ),
            throwsA(isA<ManagedProjectVerificationException>()),
            reason: mismatch,
          );
        } else {
          final published = await session.prepareAndPublishQuestDraftV3(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000071',
            scriptModuleId: '00000000000000000000000000000072',
            displayName: 'Managed Quest 1',
            intent: _questIntent(1),
          );
          expect(session.head.canonicalJson, published.head.canonicalJson);
        }
        await session.close();
      }
    },
  );

  test(
    'Quest semantic rejection is retryable while native integrity errors poison',
    () async {
      final semanticRoot = await _projectRoot(
        fixture,
        suffix: 'quest_semantic',
      );
      final semanticStore = _FakeRevision3Store();
      final semanticSession =
          await ManagedRevision3AuthoringProjectSession.create(
            root: semanticRoot,
            store: semanticStore,
            projectJson: _projectJson(revision: 0, name: 'Quest semantic'),
          );
      final semanticHead = await semanticSession.headFile.readAsBytes();
      semanticStore.nextQuestError = const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_draft_v3',
        code: 'AUTHORING_REVISION3_QUEST_REJECTED',
        message: 'fake semantic collision',
      );
      await expectLater(
        semanticSession.prepareAndPublishQuestDraftV3(
          gameRoot: r'D:\Games\Gothic Remake',
          questId: '00000000000000000000000000000071',
          scriptModuleId: '00000000000000000000000000000072',
          displayName: 'Managed Quest 1',
          intent: _questIntent(1),
        ),
        throwsA(
          isA<ModFfiException>().having(
            (error) => error.code,
            'code',
            'AUTHORING_REVISION3_QUEST_REJECTED',
          ),
        ),
      );
      expect(await semanticSession.headFile.readAsBytes(), semanticHead);
      expect(semanticSession.requiresReopen, isFalse);
      await semanticSession.prepareAndPublishQuestDraftV3(
        gameRoot: r'D:\Games\Gothic Remake',
        questId: '00000000000000000000000000000071',
        scriptModuleId: '00000000000000000000000000000072',
        displayName: 'Managed Quest 1',
        intent: _questIntent(1),
      );
      await semanticSession.close();

      for (final errorCode in <String>[
        'AUTHORING_REVISION3_QUEST_HEAD_CONFLICT',
        'AUTHORING_REVISION3_QUEST_STORE_SEAL_MISMATCH',
        'AUTHORING_REVISION3_QUEST_INVARIANT',
        'AUTHORING_REVISION3_QUEST_TRANSACTION_FAILED',
        'AUTHORING_REVISION3_QUEST_PERSISTENCE_FAILED',
        ModFfiException.malformedNativeResponseCode,
        'AUTHORING_REVISION3_QUEST_FUTURE_UNKNOWN',
      ]) {
        final root = await _projectRoot(
          fixture,
          suffix: errorCode.toLowerCase(),
        );
        final store = _FakeRevision3Store();
        final session = await ManagedRevision3AuthoringProjectSession.create(
          root: root,
          store: store,
          projectJson: _projectJson(revision: 0, name: errorCode),
        );
        final exactHead = await session.headFile.readAsBytes();
        store.nextQuestError = ModFfiException(
          command: 'authoring_store_prepare_revision3_quest_draft_v3',
          code: errorCode,
          message: 'fake integrity failure',
        );
        await expectLater(
          session.prepareAndPublishQuestDraftV3(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000071',
            scriptModuleId: '00000000000000000000000000000072',
            displayName: 'Managed Quest 1',
            intent: _questIntent(1),
          ),
          throwsA(
            errorCode.endsWith('HEAD_CONFLICT')
                ? isA<ManagedProjectHeadConflictException>()
                : isA<ManagedProjectVerificationException>(),
          ),
          reason: errorCode,
        );
        expect(await session.headFile.readAsBytes(), exactHead);
        expect(session.requiresReopen, isTrue);
        final questCalls = store.questPrepareCalls;
        await expectLater(
          session.prepareAndPublishQuestDraftV3(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000073',
            scriptModuleId: '00000000000000000000000000000074',
            displayName: 'Managed Quest 2',
            intent: _questIntent(2),
          ),
          throwsA(isA<ManagedProjectVerificationException>()),
        );
        expect(store.questPrepareCalls, questCalls);
        await session.close();
      }
    },
  );

  test(
    'head drift during native Quest prepare never clobbers the winner',
    () async {
      final root = await _projectRoot(fixture, suffix: 'quest_race');
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Quest race'),
      );
      final externalProject = _projectJson(
        revision: 90,
        name: 'External winner',
      );
      final externalHead = store.register(externalProject);
      store.afterQuestPrepare = (rootPath, _, _, _) => File(
        p.join(rootPath, 'gore-project.json'),
      ).writeAsString(externalHead.canonicalJson, flush: true);

      await expectLater(
        session.prepareAndPublishQuestDraftV3(
          gameRoot: r'D:\Games\Gothic Remake',
          questId: '00000000000000000000000000000071',
          scriptModuleId: '00000000000000000000000000000072',
          displayName: 'Managed Quest 1',
          intent: _questIntent(1),
        ),
        throwsA(isA<ManagedProjectHeadConflictException>()),
      );
      expect(await session.headFile.readAsString(), externalHead.canonicalJson);
      expect(session.projectRevision, 0);
      expect(session.requiresReopen, isTrue);
      await session.close();

      final reopened = await ManagedRevision3AuthoringProjectSession.open(
        root: root,
        store: store,
      );
      expect(reopened.projectJson, externalProject);
      await reopened.close();
    },
  );

  test(
    'derive callback cannot re-enter save, derive, verify, or close',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Original'),
      );

      final result = await session.deriveAndSave<String>((_) async {
        await expectLater(
          session.save(_projectJson(revision: 1, name: 'Nested')),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.deriveAndSave<void>(
            (_) => const ManagedProjectDerivedRejection<void>(null),
          ),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.verifyCurrentHead(),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.readContentIndex(),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.prepareAndPublishQuestDraftV3(
            gameRoot: r'D:\Games\Gothic Remake',
            questId: '00000000000000000000000000000071',
            scriptModuleId: '00000000000000000000000000000072',
            displayName: 'Managed Quest 1',
            intent: _questIntent(1),
          ),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.listDataAssetStagesV1(),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.prepareAndPublishDataAssetStageV1(
            patchReceiptPath: r'C:\fixtures\patch-receipt.json',
          ),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.prepareAndPublishRemoveDataAssetStageV1(
            targetPath: revision3DataAssetTargetPath,
          ),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        await expectLater(
          session.close(),
          throwsA(isA<ManagedProjectReentrantOperationException>()),
        );
        return const ManagedProjectDerivedRejection<String>('closed');
      });
      expect(result, 'closed');
      expect(session.isClosed, isFalse);
      await session.close();
    },
  );

  test(
    'close waits for prior work, rejects new work, and releases lock',
    () async {
      final root = await _projectRoot(fixture);
      final store = _FakeRevision3Store();
      final session = await ManagedRevision3AuthoringProjectSession.create(
        root: root,
        store: store,
        projectJson: _projectJson(revision: 0, name: 'Original'),
      );
      final entered = Completer<void>();
      final release = Completer<void>();
      store.afterPrepare = (_, _, _) async {
        entered.complete();
        await release.future;
      };
      final save = session.save(_projectJson(revision: 1, name: 'Saved'));
      await entered.future;
      final close = session.close();
      await expectLater(
        session.save(_projectJson(revision: 2, name: 'Too late')),
        throwsA(isA<ManagedProjectSessionClosedException>()),
      );
      expect(session.isClosed, isFalse);
      release.complete();
      await save;
      await close;
      expect(session.isClosed, isTrue);

      final lock = await ManagedProjectSessionLock.acquire(root);
      await lock.release();
    },
  );

  test('one managed root has one exclusive R3 session', () async {
    final root = await _projectRoot(fixture);
    final store = _FakeRevision3Store();
    final first = await ManagedRevision3AuthoringProjectSession.create(
      root: root,
      store: store,
      projectJson: _projectJson(revision: 0, name: 'Original'),
    );

    await expectLater(
      ManagedRevision3AuthoringProjectSession.open(root: root, store: store),
      throwsA(isA<ManagedProjectLockException>()),
    );
    await first.close();
  });
}

Future<ManagedRevision3QuestContextEditCheckpoint> _publishQuestContext(
  ManagedRevision3AuthoringProjectSession session,
  Revision3QuestOutlineFixture fixture,
) => session.prepareAndPublishQuestContextEditV1(
  gameRoot: r'D:\Games\Gothic Remake',
  questId: revision3QuestOutlineQuestId,
  expectedQuestRevision: fixture.questRevision,
  expectedModuleId: revision3QuestOutlineModuleId,
  expectedModuleRevision: fixture.moduleRevision,
  expectedStoryCatalogSeal: fixture.storyCatalogSeal,
  description: 'Find Homer and report back safely.',
  parentCatalogId: revision3QuestContextParentCatalogId,
  giverCatalogId: revision3QuestContextGiverCatalogId,
  expectedParentRuntimeClass: revision3QuestContextParentRuntimeClass,
  expectedParentCatalogLayer: 'base-game.quest-parent.v1',
  expectedParentAuthoringSelector: 'SwampCamp_SCChapter3',
  expectedParentSourceSeal: _contextSourceSeal(11, '1'),
  expectedGiverRuntimeUniqueName: revision3QuestContextGiverRuntimeUniqueName,
  expectedGiverCatalogLayer: 'base-game.npc.v1',
  expectedGiverAuthoringSelector: revision3QuestContextGiverRuntimeUniqueName,
  expectedGiverSourceSeal: _contextSourceSeal(12, '2'),
);

AuthoringDraftContentSeal _contextSourceSeal(int bytes, String digit) =>
    AuthoringDraftContentSeal.fromJson(<String, Object?>{
      'byte_len': bytes,
      'sha256': List<String>.filled(64, digit).join(),
    });

typedef _AfterPrepare =
    FutureOr<void> Function(
      String root,
      AuthoringWorkingHead head,
      String projectJson,
    );

typedef _AfterQuestPrepare =
    FutureOr<void> Function(
      String root,
      AuthoringWorkingHead basisHead,
      AuthoringWorkingHead candidateHead,
      String candidateProjectJson,
    );

typedef _AfterNpcPrepare =
    FutureOr<void> Function(
      String root,
      AuthoringWorkingHead basisHead,
      AuthoringWorkingHead candidateHead,
      String candidateProjectJson,
    );

typedef _AfterContentRead =
    FutureOr<void> Function(
      String root,
      AuthoringWorkingHead expectedHead,
      String projectJson,
    );

typedef _AfterQuestInspection =
    FutureOr<void> Function(
      String root,
      AuthoringWorkingHead expectedHead,
      String projectJson,
    );

typedef _AfterDataAssetPrepare =
    FutureOr<void> Function(
      String root,
      AuthoringWorkingHead basisHead,
      AuthoringWorkingHead candidateHead,
    );

typedef _AfterDataAssetList =
    FutureOr<void> Function(String root, AuthoringWorkingHead expectedHead);

typedef _AfterVoiceBuild =
    FutureOr<void> Function(
      String root,
      AuthoringRevision3VoiceBuildResult result,
    );

DataAssetSemanticEditIntent _semanticDataAssetIntent() {
  final response = validDataAssetInspectionResponse();
  (response['binding']! as Map<String, Object?>)['usmap_sha256'] = '3' * 64;
  dataAssetSelector(response)['usmap_sha256'] = '3' * 64;
  final inspection = DataAssetInspection.fromJson(response);
  return DataAssetSemanticValueEditor.fromLeaf(
        inspection.exports.single.leaves.single,
      )
      .previewScalar(
        extractReceiptPath: r'C:\proof\extract-receipt.v2.json',
        expectedTargetPath: revision3DataAssetTargetPath,
        value: '2',
      )
      .intent;
}

final class _FakeRevision3Store implements ManagedRevision3AuthoringStore {
  _FakeRevision3Store({this.sealRegisteredHeads = false});

  final bool sealRegisteredHeads;
  final Map<String, String> _projectsByHead = <String, String>{};
  final List<AuthoringAssetVerification> openVerifications =
      <AuthoringAssetVerification>[];
  final List<AuthoringAssetVerification> headVerifications =
      <AuthoringAssetVerification>[];
  final List<String?> expectedHeads = <String?>[];
  int _sequence = 0;
  int prepareCalls = 0;
  int questPrepareCalls = 0;
  int questOutlinePrepareCalls = 0;
  int questTransitionsPrepareCalls = 0;
  int questContextPrepareCalls = 0;
  int npcPrepareCalls = 0;
  int voicePrepareCalls = 0;
  int voiceSelectionPrepareCalls = 0;
  int voiceTargetPrepareCalls = 0;
  int voiceBuildCalls = 0;
  int contentReadCalls = 0;
  int questInspectionCalls = 0;
  int dataAssetPrepareCalls = 0;
  int dataAssetEditPrepareCalls = 0;
  int dataAssetListCalls = 0;
  int dataAssetRemoveCalls = 0;
  _AfterPrepare? afterPrepare;
  _AfterQuestPrepare? afterQuestPrepare;
  _AfterNpcPrepare? afterNpcPrepare;
  _AfterContentRead? afterContentRead;
  _AfterQuestInspection? afterQuestInspection;
  _AfterDataAssetPrepare? afterDataAssetPrepare;
  _AfterDataAssetList? afterDataAssetList;
  _AfterVoiceBuild? afterVoiceBuild;
  final List<String> questGameRoots = <String>[];
  final List<String> questCurrentProjects = <String>[];
  final List<AuthoringRevision3QuestDraftRequestV3> questRequests =
      <AuthoringRevision3QuestDraftRequestV3>[];
  final List<AuthoringRevision3QuestTransitionsEditRequestV1>
  questTransitionsRequests =
      <AuthoringRevision3QuestTransitionsEditRequestV1>[];
  final List<String> npcGameRoots = <String>[];
  final List<String> npcCurrentProjects = <String>[];
  final List<AuthoringRevision3NpcDraftRequestV1> npcRequests =
      <AuthoringRevision3NpcDraftRequestV1>[];
  final List<String> voiceSources = <String>[];
  final List<String> voiceGameRoots = <String>[];
  final List<String> voiceCurrentProjects = <String>[];
  final List<AuthoringRevision3VoiceTakeRequestV1> voiceRequests =
      <AuthoringRevision3VoiceTakeRequestV1>[];
  final List<AuthoringRevision3VoiceTakeSelectionRequestV1>
  voiceSelectionRequests = <AuthoringRevision3VoiceTakeSelectionRequestV1>[];
  final List<AuthoringRevision3VoiceTargetRequestV1> voiceTargetRequests =
      <AuthoringRevision3VoiceTargetRequestV1>[];
  final List<String> voiceBuildOutputs = <String>[];
  final List<String> voiceBuildGameRoots = <String>[];
  String? nextQuestResponseMismatch;
  ModFfiException? nextQuestError;
  ModFfiException? nextQuestOutlineError;
  ModFfiException? nextQuestTransitionsError;
  ModFfiException? nextQuestContextError;
  String? nextQuestContextProvenanceMismatch;
  Object? nextNpcError;
  Object? nextVoiceError;
  Object? nextVoiceSelectionError;
  Object? nextVoiceTargetError;
  Object? nextVoiceBuildError;
  ModFfiException? nextContentError;
  String? nextContentResponseMismatch;
  Object? nextQuestInspectionError;
  String? nextQuestInspectionResponseMismatch;
  Object? nextDataAssetError;
  String? nextDataAssetResponseMismatch;
  final List<String> contentExpectedHeads = <String>[];
  final List<String> questInspectionRoots = <String>[];
  final List<String> questInspectionGameRoots = <String>[];
  final List<String> questInspectionExpectedHeads = <String>[];
  final List<String> questInspectionQuestIds = <String>[];
  String? nextOpenProjectOverride;
  AuthoringWorkingHead? nextHeadOverride;
  String? nextHeadProjectOverride;
  final Map<String, Revision3DataAssetFixture> _dataAssetByHead =
      <String, Revision3DataAssetFixture>{};
  final List<DataAssetSemanticEditIntent> dataAssetEditIntents =
      <DataAssetSemanticEditIntent>[];

  AuthoringWorkingHead register(String projectJson) {
    _sequence++;
    final sha = sealRegisteredHeads
        ? crypto.sha256.convert(utf8.encode(projectJson)).toString()
        : _sequence.toRadixString(16).padLeft(64, '0');
    final head = AuthoringWorkingHead.fromCanonicalJson(
      jsonEncode(<String, Object?>{
        'store_format': 1,
        'snapshot': <String, Object?>{
          'byte_len': utf8.encode(projectJson).length,
          'sha256': sha,
        },
      }),
    );
    _projectsByHead[head.canonicalJson] = projectJson;
    return head;
  }

  @override
  Future<AuthoringRevision3StoreOpenedResult> open({
    required String root,
    required AuthoringAssetVerification verification,
  }) async {
    openVerifications.add(verification);
    final rawHead = await File(
      p.join(root, 'gore-project.json'),
    ).readAsString();
    final head = AuthoringWorkingHead.fromCanonicalJson(rawHead);
    final project = _projectsByHead[rawHead];
    if (project == null) throw StateError('unknown published head');
    final override = nextOpenProjectOverride;
    nextOpenProjectOverride = null;
    return AuthoringRevision3StoreOpenedResult.fromJson(
      _openedResponse(head, override ?? project),
    );
  }

  @override
  Future<AuthoringRevision3StoreOpenedResult> openHeadBytes({
    required String root,
    required AuthoringWorkingHead head,
    required AuthoringAssetVerification verification,
  }) async {
    headVerifications.add(verification);
    final project = _projectsByHead[head.canonicalJson];
    if (project == null) throw StateError('unknown checkpoint head');
    final headOverride = nextHeadOverride;
    nextHeadOverride = null;
    final projectOverride = nextHeadProjectOverride;
    nextHeadProjectOverride = null;
    return AuthoringRevision3StoreOpenedResult.fromJson(
      _openedResponse(headOverride ?? head, projectOverride ?? project),
    );
  }

  @override
  Future<AuthoringRevision3CheckpointPreparation> prepareCheckpoint({
    required String root,
    required AuthoringWorkingHead? expectedHead,
    required String projectJson,
  }) async {
    prepareCalls++;
    expectedHeads.add(expectedHead?.canonicalJson);
    final headFile = File(p.join(root, 'gore-project.json'));
    final actual = await headFile.exists()
        ? await headFile.readAsString()
        : null;
    if (actual != expectedHead?.canonicalJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_checkpoint',
        code: 'AUTHORING_STORE_HEAD_CONFLICT',
        message: 'fake native head CAS rejected',
      );
    }
    final head = register(projectJson);
    final hook = afterPrepare;
    afterPrepare = null;
    await hook?.call(root, head, projectJson);
    return AuthoringRevision3CheckpointPreparation.fromJson(
      _preparedResponse(head),
    );
  }

  @override
  Future<AuthoringRevision3QuestDraftPreparation> prepareQuestDraftV3({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required String questRequestJson,
  }) async {
    questPrepareCalls++;
    questGameRoots.add(gameRoot);
    questCurrentProjects.add(currentProjectJson);
    final request = AuthoringRevision3QuestDraftRequestV3.fromCanonicalJson(
      questRequestJson,
    );
    questRequests.add(request);
    final injectedError = nextQuestError;
    nextQuestError = null;
    if (injectedError != null) throw injectedError;

    final headFile = File(p.join(root, 'gore-project.json'));
    final actual = await headFile.readAsString();
    if (actual != request.expectedHead.canonicalJson ||
        _projectsByHead[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_draft_v3',
        code: 'AUTHORING_REVISION3_QUEST_HEAD_CONFLICT',
        message: 'fake native Quest basis CAS rejected',
      );
    }
    final basis = jsonDecode(currentProjectJson) as Map<String, Object?>;
    if (basis['project_id'] != request.expectedProjectId ||
        basis['revision'] != request.expectedRevision) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_draft_v3',
        code: 'AUTHORING_REVISION3_QUEST_HEAD_CONFLICT',
        message: 'fake native Quest project binding rejected',
      );
    }
    final projectId = request.expectedProjectId;
    final rawEntities = (basis['entities'] as Map).cast<String, Object?>();
    final entities = SplayTreeMap<String, Object?>.from(rawEntities);
    final questInput = _questInput(
      request: request,
      basisHead: request.expectedHead,
      target: (basis['target'] as Map).cast<String, Object?>(),
    );
    entities[request.questId] = _questEntity(
      projectId: projectId,
      request: request,
      input: questInput,
    );
    entities[request.scriptModuleId] = _questModuleEntity(
      projectId: projectId,
      request: request,
      input: questInput,
    );
    basis['revision'] = request.expectedRevision + 1;
    basis['entities'] = entities;
    final assetStore = (basis['asset_store'] as Map).cast<String, Object?>();
    final assets = SplayTreeMap<String, Object?>.from(
      (assetStore['assets'] as Map).cast<String, Object?>(),
    );
    assets[_questArtifactSha] = <String, Object?>{
      'byte_len': 123,
      'media_type':
          'application/vnd.gore.quest-collision-capability+json;version=2',
    };
    assetStore['assets'] = assets;
    basis['asset_store'] = assetStore;
    var candidateProject = jsonEncode(basis);
    var candidateHead = register(candidateProject);
    final hook = afterQuestPrepare;
    afterQuestPrepare = null;
    await hook?.call(
      root,
      request.expectedHead,
      candidateHead,
      candidateProject,
    );

    final mismatch = nextQuestResponseMismatch;
    nextQuestResponseMismatch = null;
    var basisHead = request.expectedHead;
    var responseRevision = request.expectedRevision + 1;
    var responseQuestId = request.questId;
    var responseModuleId = request.scriptModuleId;
    if (mismatch == 'basis-head') {
      basisHead = register(_projectJson(revision: 81, name: 'Wrong basis'));
    } else if (mismatch == 'candidate-project') {
      candidateProject = candidateProject.replaceAll(
        projectId,
        '00000000000000000000000000000093',
      );
      candidateHead = register(candidateProject);
    } else if (mismatch == 'revision') {
      responseRevision++;
    } else if (mismatch == 'display-name') {
      candidateProject = candidateProject.replaceFirst(
        '"display_name":"${request.displayName}"',
        '"display_name":"Wrong prepared Quest"',
      );
      candidateHead = register(candidateProject);
    } else if (mismatch == 'quest-id') {
      responseQuestId = request.scriptModuleId;
    } else if (mismatch == 'module-id') {
      responseModuleId = request.questId;
    }
    try {
      return AuthoringRevision3QuestDraftPreparation.fromJson(
        _questPreparedResponse(
          basisHead: basisHead,
          candidateHead: candidateHead,
          candidateProjectJson: candidateProject,
          revision: responseRevision,
          questId: responseQuestId,
          scriptModuleId: responseModuleId,
        ),
      );
    } on FormatException catch (error) {
      if (mismatch == null) rethrow;
      throw ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_draft_v3',
        code: ModFfiException.malformedNativeResponseCode,
        message: error.message.toString(),
      );
    }
  }

  @override
  Future<AuthoringRevision3QuestOutlineEditPreparation>
  prepareQuestOutlineEditV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestOutlineEditRequestV1 request,
  }) async {
    questOutlinePrepareCalls++;
    final injected = nextQuestOutlineError;
    nextQuestOutlineError = null;
    if (injected != null) throw injected;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != request.expectedHead.canonicalJson ||
        _projectsByHead[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_outline_edit_v1',
        code: 'AUTHORING_REVISION3_QUEST_OUTLINE_HEAD_CONFLICT',
        message: 'fake native Quest outline basis CAS rejected',
      );
    }
    final fixture = Revision3QuestOutlineFixture();
    if (currentProjectJson != fixture.projectJson) {
      throw StateError('fake outline fixture received an unexpected basis');
    }
    final response = fixture.response(
      displayName: request.displayName,
      title: request.title,
      objectiveTitles: request.objectiveTitles,
    );
    response['basis_head_json'] = request.expectedHead.canonicalJson;
    final candidateProject = response['project_json']! as String;
    final candidateHead = register(candidateProject);
    response['head_json'] = candidateHead.canonicalJson;
    return AuthoringRevision3QuestOutlineEditPreparation.fromJson(
      response,
      currentProjectJson: currentProjectJson,
      request: request,
    );
  }

  @override
  Future<AuthoringRevision3QuestTransitionsEditPreparation>
  prepareQuestTransitionsEditV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3QuestTransitionsEditRequestV1 request,
  }) async {
    questTransitionsPrepareCalls++;
    questTransitionsRequests.add(request);
    final injected = nextQuestTransitionsError;
    nextQuestTransitionsError = null;
    if (injected != null) throw injected;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != request.expectedHead.canonicalJson ||
        _projectsByHead[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_transitions_edit_v1',
        code: 'AUTHORING_REVISION3_QUEST_TRANSITIONS_HEAD_CONFLICT',
        message: 'fake native Quest transitions basis CAS rejected',
      );
    }
    final fixture = Revision3QuestOutlineFixture();
    if (currentProjectJson != fixture.projectJson) {
      throw StateError('fake transitions fixture received an unexpected basis');
    }
    final candidate = fixture.projectObject();
    candidate['revision'] = fixture.projectRevision + 1;
    final entities = (candidate['entities']! as Map).cast<String, Object?>();
    final quest = (entities[revision3QuestOutlineQuestId]! as Map)
        .cast<String, Object?>();
    quest['revision'] = fixture.questRevision + 1;
    final questPayload = (quest['payload']! as Map).cast<String, Object?>();
    final questData = (questPayload['data']! as Map).cast<String, Object?>();
    questData['generator_version'] = 4;
    final input = (questData['input']! as Map).cast<String, Object?>();
    input['transition_plan'] = request.transitionPlan.toJson();

    final module = (entities[revision3QuestOutlineModuleId]! as Map)
        .cast<String, Object?>();
    module['revision'] = fixture.moduleRevision + 1;
    final origin = (module['origin']! as Map).cast<String, Object?>();
    origin['generator_version'] = 4;
    final modulePayload = (module['payload']! as Map).cast<String, Object?>();
    final moduleData = (modulePayload['data']! as Map).cast<String, Object?>();
    moduleData['generator_version'] = 4;
    moduleData['input_fingerprint'] = revision3QuestInputFingerprint(input);

    final candidateProject = jsonEncode(candidate);
    final candidateHead = register(candidateProject);
    final response = <String, Object?>{
      'ok': true,
      'outcome': 'prepared_unpublished',
      'basis_head_json': request.expectedHead.canonicalJson,
      'head_json': candidateHead.canonicalJson,
      'project_json': candidateProject,
      'project_id': revision3QuestOutlineProjectId,
      'revision': fixture.projectRevision + 1,
      'quest_id': revision3QuestOutlineQuestId,
      'module_id': revision3QuestOutlineModuleId,
      'quest_revision': fixture.questRevision + 1,
      'module_revision': fixture.moduleRevision + 1,
      'previous_generator_version': request.previousGeneratorVersion,
      'upgraded_from_legacy': request.upgradesLegacy,
      'transition_plan_seal': <String, Object?>{
        'byte_len': request.transitionPlan.contentSeal.byteLength,
        'sha256': request.transitionPlan.contentSeal.sha256,
      },
      'build_status': 'blocked',
      'runtime_status': 'runtime_unqualified',
      'publication_status': 'not_supported',
    };
    return AuthoringRevision3QuestTransitionsEditPreparation.fromJson(
      response,
      currentProjectJson: currentProjectJson,
      request: request,
    );
  }

  @override
  Future<AuthoringRevision3QuestContextEditPreparation>
  prepareQuestContextEditV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3QuestContextEditRequestV1 request,
  }) async {
    questContextPrepareCalls++;
    final injected = nextQuestContextError;
    nextQuestContextError = null;
    if (injected != null) throw injected;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != request.expectedHead.canonicalJson ||
        _projectsByHead[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_context_edit_v1',
        code: 'AUTHORING_REVISION3_QUEST_CONTEXT_HEAD_CONFLICT',
        message: 'fake native Quest context basis CAS rejected',
      );
    }
    final fixture = Revision3QuestOutlineFixture();
    if (currentProjectJson != fixture.projectJson) {
      throw StateError('fake context fixture received an unexpected basis');
    }
    final mismatch = nextQuestContextProvenanceMismatch;
    nextQuestContextProvenanceMismatch = null;
    final response = fixture.contextResponse(
      description: request.description,
      parentCatalogId: request.parentCatalogId,
      giverCatalogId: request.giverCatalogId,
      parentCatalogLayer: mismatch == 'layer'
          ? 'base-game.quest-parent.wrong'
          : 'base-game.quest-parent.v1',
      parentAuthoringSelector: mismatch == 'selector'
          ? 'WrongSameRuntimeSelector'
          : 'SwampCamp_SCChapter3',
      parentSourceSeal: mismatch == 'seal'
          ? <String, Object?>{
              'byte_len': 99,
              'sha256': List<String>.filled(64, '8').join(),
            }
          : null,
    );
    response['basis_head_json'] = request.expectedHead.canonicalJson;
    final candidateProject = response['project_json']! as String;
    final candidateHead = register(candidateProject);
    response['head_json'] = candidateHead.canonicalJson;
    try {
      return AuthoringRevision3QuestContextEditPreparation.fromJson(
        response,
        currentProjectJson: currentProjectJson,
        request: request,
      );
    } on FormatException catch (error) {
      throw ModFfiException(
        command: 'authoring_store_prepare_revision3_quest_context_edit_v1',
        code: ModFfiException.malformedNativeResponseCode,
        message: error.message.toString(),
      );
    }
  }

  @override
  Future<AuthoringRevision3NpcDraftPreparation> prepareNpcDraftV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3NpcDraftRequestV1 request,
  }) async {
    npcPrepareCalls++;
    npcGameRoots.add(gameRoot);
    npcCurrentProjects.add(currentProjectJson);
    npcRequests.add(request);
    final injectedError = nextNpcError;
    nextNpcError = null;
    if (injectedError != null) throw injectedError;

    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != request.expectedHead.canonicalJson ||
        _projectsByHead[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_npc_draft_v1',
        code: 'AUTHORING_REVISION3_NPC_HEAD_CONFLICT',
        message: 'fake native NPC basis CAS rejected',
      );
    }
    final fixture = Revision3NpcFixture.fromBasis(
      basisHead: request.expectedHead,
      basisProjectJson: currentProjectJson,
      request: request,
    );
    _projectsByHead[fixture.candidateHead.canonicalJson] =
        fixture.candidateProjectJson;
    final hook = afterNpcPrepare;
    afterNpcPrepare = null;
    await hook?.call(
      root,
      request.expectedHead,
      fixture.candidateHead,
      fixture.candidateProjectJson,
    );
    return AuthoringRevision3NpcDraftPreparation.fromJson(
      fixture.response(),
      currentProjectJson: currentProjectJson,
      request: request,
    );
  }

  @override
  Future<AuthoringRevision3VoiceTakePreparation> prepareVoiceTakeV1({
    required String root,
    required String gameRoot,
    required String source,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeRequestV1 request,
  }) async {
    voicePrepareCalls++;
    voiceGameRoots.add(gameRoot);
    voiceSources.add(source);
    voiceCurrentProjects.add(currentProjectJson);
    voiceRequests.add(request);
    final injectedError = nextVoiceError;
    nextVoiceError = null;
    if (injectedError != null) throw injectedError;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != request.expectedHead.canonicalJson ||
        _projectsByHead[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_voice_take_v1',
        code: 'AUTHORING_REVISION3_VOICE_HEAD_CONFLICT',
        message: 'fake native Voice basis CAS rejected',
      );
    }
    final fixture = Revision3VoiceFixture.fromBasis(
      basisHead: request.expectedHead,
      basisProjectJson: currentProjectJson,
      request: request,
    );
    _projectsByHead[fixture.candidateHead.canonicalJson] =
        fixture.candidateProjectJson;
    return AuthoringRevision3VoiceTakePreparation.fromJson(
      fixture.response(),
      currentProjectJson: currentProjectJson,
      request: request,
    );
  }

  @override
  Future<AuthoringRevision3VoiceTakeSelectionPreparation>
  prepareVoiceTakeSelectionV1({
    required String root,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTakeSelectionRequestV1 request,
  }) async {
    voiceSelectionPrepareCalls++;
    voiceSelectionRequests.add(request);
    final injectedError = nextVoiceSelectionError;
    nextVoiceSelectionError = null;
    if (injectedError != null) throw injectedError;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != request.expectedHead.canonicalJson ||
        _projectsByHead[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_voice_take_selection_v1',
        code: 'AUTHORING_REVISION3_VOICE_SELECTION_HEAD_CONFLICT',
        message: 'fake native Voice selection basis CAS rejected',
      );
    }
    final candidate = (jsonDecode(currentProjectJson) as Map)
        .cast<String, Object?>();
    candidate['revision'] = request.expectedRevision + 1;
    final entities = (candidate['entities']! as Map).cast<String, Object?>();
    final slot = (entities[request.slotId]! as Map).cast<String, Object?>();
    slot['revision'] = request.expectedSlotRevision + 1;
    final payload = (slot['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();
    final selected = request.selectedTakeId;
    if (selected == null) {
      data.remove('selected');
    } else {
      data['selected'] = <String, Object?>{
        'project_id': request.expectedProjectId,
        'id': selected,
        'expected_kind': 'voice_take',
      };
    }
    payload['data'] = data;
    slot['payload'] = payload;
    entities[request.slotId] = slot;
    candidate['entities'] = entities;
    final candidateProject = jsonEncode(candidate);
    final candidateHead = register(candidateProject);
    return AuthoringRevision3VoiceTakeSelectionPreparation.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'prepared_unpublished',
        'basis_head_json': request.expectedHead.canonicalJson,
        'head_json': candidateHead.canonicalJson,
        'project_json': candidateProject,
        'project_id': request.expectedProjectId,
        'revision': request.expectedRevision + 1,
        'line_id': request.lineId,
        'slot_id': request.slotId,
        'slot_revision': request.expectedSlotRevision + 1,
        'locale': request.locale,
        'loc_id': request.expectedLocId,
        'previous_selected_take_id': request.expectedSelectedTakeId,
        'selected_take_id': request.selectedTakeId,
        'build_status': 'blocked',
        'runtime_status': 'runtime_unqualified',
        'publication_status': 'not_supported',
      },
      currentProjectJson: currentProjectJson,
      request: request,
    );
  }

  @override
  Future<AuthoringRevision3VoiceTargetPreparation> prepareVoiceTargetV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringRevision3VoiceTargetRequestV1 request,
  }) async {
    voiceTargetPrepareCalls++;
    voiceTargetRequests.add(request);
    final injectedError = nextVoiceTargetError;
    nextVoiceTargetError = null;
    if (injectedError != null) throw injectedError;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != request.expectedHead.canonicalJson ||
        _projectsByHead[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_voice_target_v1',
        code: 'AUTHORING_REVISION3_VOICE_TARGET_HEAD_CONFLICT',
        message: 'fake native Voice target basis CAS rejected',
      );
    }
    final target = <String, Object?>{
      'archive': 'german_new.zip',
      'member': 'Voices/Hero/${request.expectedLocId}.ogg',
      'operation': 'replace',
      'archive_seal': <String, Object?>{'byte_len': 4096, 'sha256': 'c' * 64},
      'member_proof': <String, Object?>{
        'state': 'present',
        'uncompressed_size': 8192,
        'crc32': 42,
      },
    };
    final resolution = <String, Object?>{'state': 'resolved', 'target': target};
    final candidate = (jsonDecode(currentProjectJson) as Map)
        .cast<String, Object?>();
    candidate['revision'] = request.expectedRevision + 1;
    final entities = (candidate['entities']! as Map).cast<String, Object?>();
    final slot = (entities[request.slotId]! as Map).cast<String, Object?>();
    slot['revision'] = (slot['revision']! as int) + 1;
    final payload = (slot['payload']! as Map).cast<String, Object?>();
    final data = (payload['data']! as Map).cast<String, Object?>();
    data['target_resolution'] = resolution;
    payload['data'] = data;
    slot['payload'] = payload;
    entities[request.slotId] = slot;
    candidate['entities'] = entities;
    final candidateProjectJson = jsonEncode(candidate);
    final candidateHead = register(candidateProjectJson);
    return AuthoringRevision3VoiceTargetPreparation.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'prepared_unpublished',
        'basis_head_json': request.expectedHead.canonicalJson,
        'head_json': candidateHead.canonicalJson,
        'project_json': candidateProjectJson,
        'revision': request.expectedRevision + 1,
        'line_id': request.lineId,
        'localization_id': revision3VoiceFixtureLocalizationId,
        'slot_id': request.slotId,
        'locale': request.locale,
        'loc_id': request.expectedLocId,
        'resolution': 'resolved',
        'match_count': 1,
        'target_resolution': resolution,
        'archive_observation': <String, Object?>{
          'archive': 'german_new.zip',
          'archive_seal': <String, Object?>{
            'byte_len': 4096,
            'sha256': 'c' * 64,
          },
        },
        'build_status': 'blocked',
        'runtime_status': 'runtime_unqualified',
        'publication_status': 'not_supported',
      },
      currentProjectJson: currentProjectJson,
      request: request,
    );
  }

  @override
  Future<AuthoringRevision3VoiceBuildResult> buildVoiceV1({
    required String root,
    required String gameRoot,
    required String currentProjectJson,
    required AuthoringWorkingHead expectedHead,
    required String output,
  }) async {
    voiceBuildCalls++;
    voiceBuildGameRoots.add(gameRoot);
    voiceBuildOutputs.add(output);
    final injectedError = nextVoiceBuildError;
    nextVoiceBuildError = null;
    if (injectedError != null) throw injectedError;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != expectedHead.canonicalJson ||
        _projectsByHead[actual] != currentProjectJson) {
      throw const ModFfiException(
        command: 'authoring_store_build_revision3_voice_v1',
        code: 'AUTHORING_REVISION3_VOICE_BUILD_HEAD_CONFLICT',
        message: 'fake native Voice build basis CAS rejected',
      );
    }
    final project = (jsonDecode(currentProjectJson) as Map)
        .cast<String, Object?>();
    final result = AuthoringRevision3VoiceBuildResult.fromJson(
      <String, Object?>{
        'ok': true,
        'outcome': 'built',
        'basis_head_json': expectedHead.canonicalJson,
        'project_id': project['project_id'],
        'project_revision': project['revision'],
        'output': output,
        'edit_count': 1,
        'file_count': 3,
        'bundle_bytes': 1234,
        'bundle_sha256': 'd' * 64,
        'build_authority': 'generation_sealed_existing_member_bundle_v1',
        'deployment_status': 'not_performed',
      },
      expectedHead: expectedHead,
      expectedProjectJson: currentProjectJson,
      expectedOutput: output,
    );
    await afterVoiceBuild?.call(root, result);
    return result;
  }

  @override
  Future<AuthoringRevision3ContentIndexResult> readContentIndex({
    required String root,
    required AuthoringWorkingHead expectedHead,
  }) async {
    contentReadCalls++;
    contentExpectedHeads.add(expectedHead.canonicalJson);
    final injectedError = nextContentError;
    nextContentError = null;
    if (injectedError != null) throw injectedError;

    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != expectedHead.canonicalJson) {
      throw const ModFfiException(
        command: 'authoring_store_read_revision3_content_index_v1',
        code: 'AUTHORING_REVISION3_CONTENT_HEAD_CONFLICT',
        message: 'fake native content head CAS rejected',
      );
    }
    final project = _projectsByHead[actual];
    if (project == null) throw StateError('unknown content checkpoint head');
    final hook = afterContentRead;
    afterContentRead = null;
    await hook?.call(root, expectedHead, project);

    final mismatch = nextContentResponseMismatch;
    nextContentResponseMismatch = null;
    return AuthoringRevision3ContentIndexResult.fromJson(
      _contentResponse(
        expectedHead,
        project,
        responseProjectId: mismatch == 'project-id'
            ? '93939393939393939393939393939393'
            : null,
      ),
      expectedHead: expectedHead,
    );
  }

  @override
  Future<AuthoringRevision3QuestSourceInspectionResult> inspectQuestSourceV1({
    required String root,
    required String gameRoot,
    required AuthoringWorkingHead expectedHead,
    required String questId,
  }) async {
    questInspectionCalls++;
    questInspectionRoots.add(root);
    questInspectionGameRoots.add(gameRoot);
    questInspectionExpectedHeads.add(expectedHead.canonicalJson);
    questInspectionQuestIds.add(questId);
    final injectedError = nextQuestInspectionError;
    nextQuestInspectionError = null;
    if (injectedError != null) throw injectedError;

    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    if (actual != expectedHead.canonicalJson) {
      throw const ModFfiException(
        command: 'authoring_store_inspect_revision3_quest_source_v1',
        code: 'AUTHORING_REVISION3_QUEST_INSPECTION_HEAD_CONFLICT',
        message: 'fake native Quest inspection head CAS rejected',
      );
    }
    final projectJson = _projectsByHead[actual];
    if (projectJson == null) {
      throw StateError('unknown Quest inspection checkpoint head');
    }
    final hook = afterQuestInspection;
    afterQuestInspection = null;
    await hook?.call(root, expectedHead, projectJson);

    final mismatch = nextQuestInspectionResponseMismatch;
    nextQuestInspectionResponseMismatch = null;
    final project = (jsonDecode(projectJson) as Map).cast<String, Object?>();
    final responseHead = mismatch == 'head'
        ? register(projectJson)
        : expectedHead;
    final responseQuestId = mismatch == 'quest'
        ? '00000000000000000000000000000073'
        : questId;
    return _questSourceInspectionResult(
      head: responseHead,
      projectJson: projectJson,
      questId: responseQuestId,
      projectId: mismatch == 'project-id'
          ? '00000000000000000000000000000093'
          : project['project_id']! as String,
      projectRevision: mismatch == 'revision'
          ? (project['revision']! as int) + 1
          : project['revision']! as int,
      projectSealJson: mismatch == 'project-seal'
          ? _projectJson(revision: 99, name: 'Mismatched inspection seal')
          : projectJson,
    );
  }

  @override
  Future<AuthoringRevision3DataAssetStagePreparation> prepareDataAssetStageV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String patchReceiptPath,
  }) async {
    dataAssetPrepareCalls++;
    final injected = nextDataAssetError;
    nextDataAssetError = null;
    if (injected != null) throw injected;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    final project = _projectsByHead[actual];
    if (actual != expectedHead.canonicalJson || project == null) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_dataasset_stage_v1',
        code: 'AUTHORING_REVISION3_DATAASSET_HEAD_CONFLICT',
        message: 'fake native DataAsset basis CAS rejected',
      );
    }
    final fixture = Revision3DataAssetFixture.fromBasis(
      basisHead: expectedHead,
      basisProjectJson: project,
    );
    _projectsByHead[fixture.stagedHead.canonicalJson] =
        fixture.stagedProjectJson;
    _dataAssetByHead[fixture.stagedHead.canonicalJson] = fixture;
    final hook = afterDataAssetPrepare;
    afterDataAssetPrepare = null;
    await hook?.call(root, expectedHead, fixture.stagedHead);
    final response = fixture.prepareResponse();
    final mismatch = nextDataAssetResponseMismatch;
    nextDataAssetResponseMismatch = null;
    if (mismatch == 'revision') response['revision'] = 99;
    return AuthoringRevision3DataAssetStagePreparation.fromJson(
      response,
      expectedHead: expectedHead,
    );
  }

  @override
  Future<AuthoringRevision3DataAssetStagePreparation> prepareDataAssetEditV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required DataAssetSemanticEditIntent intent,
  }) async {
    dataAssetEditPrepareCalls++;
    dataAssetEditIntents.add(intent);
    final injected = nextDataAssetError;
    nextDataAssetError = null;
    if (injected != null) throw injected;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    final project = _projectsByHead[actual];
    if (actual != expectedHead.canonicalJson || project == null) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_revision3_dataasset_edit_v1',
        code: 'AUTHORING_REVISION3_DATAASSET_HEAD_CONFLICT',
        message: 'fake native DataAsset edit basis CAS rejected',
      );
    }
    final fixture = Revision3DataAssetFixture.fromBasis(
      basisHead: expectedHead,
      basisProjectJson: project,
      targetPath: intent.expectedTargetPath,
      selector: intent.selector.toJson(),
      replacementHex: _semanticReplacementHex(intent),
    );
    _projectsByHead[fixture.stagedHead.canonicalJson] =
        fixture.stagedProjectJson;
    _dataAssetByHead[fixture.stagedHead.canonicalJson] = fixture;
    final hook = afterDataAssetPrepare;
    afterDataAssetPrepare = null;
    await hook?.call(root, expectedHead, fixture.stagedHead);
    final response = fixture.prepareResponse()
      ..['intent_binding_sha256'] = intent.intentBindingSha256;
    final mismatch = nextDataAssetResponseMismatch;
    nextDataAssetResponseMismatch = null;
    if (mismatch == 'revision') response['revision'] = 99;
    if (mismatch == 'intent-binding') {
      response['intent_binding_sha256'] = 'f' * 64;
    }
    return AuthoringRevision3DataAssetStagePreparation.fromJson(
      response,
      expectedHead: expectedHead,
      expectedIntentBindingSha256: intent.intentBindingSha256,
    );
  }

  @override
  Future<AuthoringRevision3DataAssetStageListResult> listDataAssetStagesV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
  }) async {
    dataAssetListCalls++;
    final injected = nextDataAssetError;
    nextDataAssetError = null;
    if (injected != null) throw injected;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    final project = _projectsByHead[actual];
    if (actual != expectedHead.canonicalJson || project == null) {
      throw const ModFfiException(
        command: 'authoring_store_list_revision3_dataasset_stages_v1',
        code: 'AUTHORING_REVISION3_DATAASSET_HEAD_CONFLICT',
        message: 'fake native DataAsset list CAS rejected',
      );
    }
    final hook = afterDataAssetList;
    afterDataAssetList = null;
    await hook?.call(root, expectedHead);
    final fixture = _dataAssetByHead[expectedHead.canonicalJson];
    final response =
        fixture?.listResponse() ??
        <String, Object?>{
          'ok': true,
          'outcome': 'listed_exact_head',
          'basis_head_json': expectedHead.canonicalJson,
          'revision':
              (jsonDecode(project) as Map<String, Object?>)['revision']! as int,
          'stages': <Object?>[],
          'build_status': 'blocked',
          'runtime_status': 'runtime_unqualified',
          'artifact_authority': 'not_granted',
          'publication_status': 'not_supported',
        };
    final mismatch = nextDataAssetResponseMismatch;
    nextDataAssetResponseMismatch = null;
    if (mismatch == 'revision') response['revision'] = 99;
    return AuthoringRevision3DataAssetStageListResult.fromJson(
      response,
      expectedHead: expectedHead,
    );
  }

  @override
  Future<AuthoringRevision3DataAssetStageRemovalPreparation>
  prepareRemoveDataAssetStageV1({
    required String root,
    required AuthoringWorkingHead expectedHead,
    required String targetPath,
  }) async {
    dataAssetRemoveCalls++;
    final injected = nextDataAssetError;
    nextDataAssetError = null;
    if (injected != null) throw injected;
    final actual = await File(p.join(root, 'gore-project.json')).readAsString();
    final fixture = _dataAssetByHead[actual];
    if (actual != expectedHead.canonicalJson || fixture == null) {
      throw const ModFfiException(
        command: 'authoring_store_prepare_remove_revision3_dataasset_stage_v1',
        code: 'AUTHORING_REVISION3_DATAASSET_TARGET_MISSING',
        message: 'fake native DataAsset target is absent',
      );
    }
    _projectsByHead[fixture.removedHead.canonicalJson] =
        fixture.removedProjectJson;
    final hook = afterDataAssetPrepare;
    afterDataAssetPrepare = null;
    await hook?.call(root, expectedHead, fixture.removedHead);
    final response = fixture.removalResponse();
    final mismatch = nextDataAssetResponseMismatch;
    nextDataAssetResponseMismatch = null;
    if (mismatch == 'revision') response['revision'] = 99;
    return AuthoringRevision3DataAssetStageRemovalPreparation.fromJson(
      response,
      expectedHead: expectedHead,
      requestedTargetPath: targetPath,
    );
  }
}

String _semanticReplacementHex(DataAssetSemanticEditIntent intent) {
  final wire = intent.replacement.toJson();
  final kind = wire['kind']! as String;
  final bytes = switch (kind) {
    'bool' => <int>[(wire['value']! as bool) ? 1 : 0],
    'byte' => _semanticIntegerBytes(wire['decimal']! as String, 1),
    'int8' => _semanticIntegerBytes(wire['decimal']! as String, 1),
    'int16' => _semanticIntegerBytes(wire['decimal']! as String, 2),
    'int32' => _semanticIntegerBytes(wire['decimal']! as String, 4),
    'int64' => _semanticIntegerBytes(wire['decimal']! as String, 8),
    'uint16' => _semanticIntegerBytes(wire['decimal']! as String, 2),
    'uint32' => _semanticIntegerBytes(wire['decimal']! as String, 4),
    'uint64' => _semanticIntegerBytes(wire['decimal']! as String, 8),
    'float32' => _semanticFloatBytes(<String>[
      wire['decimal']! as String,
    ], singlePrecision: true),
    'float64' => _semanticFloatBytes(<String>[
      wire['decimal']! as String,
    ], singlePrecision: false),
    'linear_color_f32x4' => _semanticFloatBytes(<String>[
      wire['r']! as String,
      wire['g']! as String,
      wire['b']! as String,
      wire['a']! as String,
    ], singlePrecision: true),
    'vector4_f64x4' => _semanticFloatBytes(<String>[
      wire['x']! as String,
      wire['y']! as String,
      wire['z']! as String,
      wire['w']! as String,
    ], singlePrecision: false),
    _ => throw StateError('unsupported fake semantic replacement $kind'),
  };
  return bytes.map((byte) => byte.toRadixString(16).padLeft(2, '0')).join();
}

List<int> _semanticIntegerBytes(String decimal, int width) {
  var value = BigInt.parse(decimal);
  if (value.isNegative) value += BigInt.one << (width * 8);
  return List<int>.generate(
    width,
    (index) => ((value >> (index * 8)) & BigInt.from(0xff)).toInt(),
  );
}

List<int> _semanticFloatBytes(
  List<String> values, {
  required bool singlePrecision,
}) {
  final width = singlePrecision ? 4 : 8;
  final data = ByteData(width * values.length);
  for (var index = 0; index < values.length; index++) {
    final value = double.parse(values[index]);
    if (singlePrecision) {
      data.setFloat32(index * width, value, Endian.little);
    } else {
      data.setFloat64(index * width, value, Endian.little);
    }
  }
  return data.buffer.asUint8List();
}

Future<Directory> _projectRoot(Directory fixture, {String suffix = ''}) async {
  final root = Directory(
    p.join(fixture.path, suffix.isEmpty ? 'project' : 'project_$suffix'),
  );
  await root.create();
  return root;
}

String _projectJson({
  required int revision,
  required String name,
}) => jsonEncode(<String, Object?>{
  'format': 2,
  'schema_revision': 3,
  'project_id': '00000000000000000000000000000003',
  'revision': revision,
  'meta': <String, Object?>{
    'name': name,
    'version': '1.0.0',
    'author': 'revision-3 session tests',
  },
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 171698176,
      'sha256':
          'f406f969d3e73b6e58ea6e7aa10df7380318d97e7974d3be6e5a01183a4524f5',
    },
  },
  'authoring_locales': <Object?>[],
  'entities': <String, Object?>{},
  'asset_store': <String, Object?>{'assets': <String, Object?>{}},
});

Map<String, Object?> _contentResponse(
  AuthoringWorkingHead head,
  String projectJson, {
  String? responseProjectId,
}) {
  final project = jsonDecode(projectJson) as Map<String, Object?>;
  final projectId = responseProjectId ?? project['project_id']! as String;
  final revision = project['revision']! as int;
  final meta = (project['meta']! as Map).cast<String, Object?>();
  final target = (project['target']! as Map).cast<String, Object?>();
  return <String, Object?>{
    'ok': true,
    'head_json': head.canonicalJson,
    'project_id': projectId,
    'project_revision': revision,
    'index_json': jsonEncode(<String, Object?>{
      'schema_revision': 1,
      'project_id': projectId,
      'project_revision': revision,
      'project_name': meta['name'],
      'project_version': meta['version'],
      'project_author': meta['author'],
      'target': target,
      'authoring_locales': project['authoring_locales'],
      'entity_counts': <String, Object?>{},
      'entities': <Object?>[],
      'assets': <Object?>[],
    }),
    'content_authority': 'read_only_exact_current_project',
    'build_status': 'not_evaluated',
    'runtime_status': 'runtime_unqualified',
    'publication_status': 'not_applicable',
  };
}

AuthoringRevision3QuestSourceInspectionResult _questSourceInspectionResult({
  required AuthoringWorkingHead head,
  required String projectJson,
  required String questId,
  required String projectId,
  required int projectRevision,
  String? projectSealJson,
}) {
  const moduleId = '00000000000000000000000000000072';
  const source = '''class UQuest_GoreInspection : UQuest
{
    void OnStart() {}
}
''';
  final sourceBytes = utf8.encode(source);
  final sourceSha = crypto.sha256.convert(sourceBytes).toString();
  final sealedProjectJson = projectSealJson ?? projectJson;
  final projectBytes = utf8.encode(sealedProjectJson);
  final projectSeal = <String, Object?>{
    'byte_len': projectBytes.length,
    'sha256': crypto.sha256.convert(projectBytes).toString(),
  };
  Map<String, Object?> seal(int byteLength, String digit) => <String, Object?>{
    'byte_len': byteLength,
    'sha256': List<String>.filled(64, digit).join(),
  };
  Map<String, Object?> typedRef(String id, String kind) => <String, Object?>{
    'project_id': projectId,
    'id': id,
    'expected_kind': kind,
  };
  final planJson = jsonEncode(<String, Object?>{
    'format': 'revision3_quest_source_inspection_plan',
    'schema_revision': 3,
    'scope': 'source_inspection_only',
    'build_status': 'blocked',
    'runtime_qualification': 'runtime_unqualified',
    'publication_status': 'not_supported',
    'provenance': <String, Object?>{
      'project_id': projectId,
      'project_revision': projectRevision,
      'target_executable': seal(171698176, '2'),
      'canonical_project': projectSeal,
      'collision_basis_head': jsonDecode(head.canonicalJson),
      'collision_basis_project': seal(1024, '3'),
      'collision_nonquest_project': seal(900, '4'),
      'collision_prior_quest_count': 2,
      'collision_prior_quest_evidence': seal(300, '5'),
      'collision_artifact': seal(700, '6'),
      'collision_source': seal(700, '7'),
    },
    'module': <String, Object?>{
      'quest': typedRef(questId, 'quest_draft'),
      'script_module': typedRef(moduleId, 'script_module'),
      'draft_input': seal(420, '8'),
      'persisted_source': <String, Object?>{
        'byte_len': sourceBytes.length,
        'sha256': sourceSha,
      },
      'generated': <String, Object?>{
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': 4,
        'owner': typedRef(questId, 'quest_draft'),
        'module_namespace': 'GoreMods.Quests.Inspection',
        'module_relative_path': 'GoreMods/Quests/Inspection.as',
        'source': source,
        'source_sha256': sourceSha,
        'input_fingerprint': List<String>.filled(64, '9').join(),
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
    },
  });
  final planBytes = utf8.encode(planJson);
  return AuthoringRevision3QuestSourceInspectionResult.fromJson(
    <String, Object?>{
      'ok': true,
      'outcome': 'inspection_only',
      'head_json': head.canonicalJson,
      'project_id': projectId,
      'project_revision': projectRevision,
      'project_seal': projectSeal,
      'quest_id': questId,
      'plan_json': planJson,
      'plan_seal': <String, Object?>{
        'byte_len': planBytes.length,
        'sha256': crypto.sha256.convert(planBytes).toString(),
      },
      'scope': 'source_inspection_only',
      'build_status': 'blocked',
      'runtime_qualification': 'runtime_unqualified',
      'publication_status': 'not_supported',
    },
    expectedHead: head,
    requestedQuestId: questId,
  );
}

const _questArtifactSha =
    'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee';

Map<String, Object?> _questSeal(int byteLength, String digit) =>
    <String, Object?>{
      'byte_len': byteLength,
      'sha256': List<String>.filled(64, digit).join(),
    };

Map<String, Object?> _questInput({
  required AuthoringRevision3QuestDraftRequestV3 request,
  required AuthoringWorkingHead basisHead,
  required Map<String, Object?> target,
}) => <String, Object?>{
  'target': target,
  'quest_id': request.questId,
  'module_namespace': request.intent.moduleNamespace,
  'technical_id': request.intent.technicalId,
  'text_helper': request.intent.textHelper,
  'parent_quest': <String, Object?>{
    'generation': target,
    'source_seal': _questSeal(11, '1'),
    'catalog_layer': 'base-game.quest-parent.v1',
    'canonical_selector': 'SwampCamp_SCChapter2',
    'runtime_class': 'UQuest_SwampCamp_SCChapter2',
  },
  'giver': <String, Object?>{
    'generation': target,
    'source_seal': _questSeal(12, '2'),
    'catalog_layer': 'base-game.npc.v1',
    'canonical_selector': 'OM_GRD_Asghan_263',
    'runtime_unique_name': 'OM_GRD_Asghan_263',
  },
  'title': request.intent.title,
  'description': request.intent.description,
  'objective_title': request.intent.objectiveTitle,
  if (request.intent.additionalObjectiveTitles.isNotEmpty)
    'additional_objective_titles': request.intent.additionalObjectiveTitles,
  'collision_catalog': <String, Object?>{
    'generation': target,
    'catalog_layer':
        'base-game-plus-exact-revision3-project.story-collisions.v2',
    'artifact': _questSeal(123, 'e'),
    'source_seal': _questSeal(123, 'f'),
    'basis_snapshot': <String, Object?>{
      'byte_len': basisHead.snapshotByteLength,
      'sha256': basisHead.snapshotSha256,
    },
  },
};

String _questInputFingerprint(Map<String, Object?> input) {
  return revision3QuestInputFingerprint(input);
}

Map<String, Object?> _questEntity({
  required String projectId,
  required AuthoringRevision3QuestDraftRequestV3 request,
  required Map<String, Object?> input,
}) {
  final generatorVersion = request.intent.additionalObjectiveTitles.isEmpty
      ? 2
      : 3;
  return <String, Object?>{
    'id': request.questId,
    'display_name': request.displayName,
    'origin': <String, Object?>{
      'type': 'new',
      'authored_runtime_id': request.intent.technicalId,
    },
    'revision': 0,
    'payload': <String, Object?>{
      'kind': 'quest_draft',
      'data': <String, Object?>{
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': generatorVersion,
        'input': input,
        'script_module': <String, Object?>{
          'project_id': projectId,
          'id': request.scriptModuleId,
          'expected_kind': 'script_module',
        },
      },
    },
  };
}

Map<String, Object?> _questModuleEntity({
  required String projectId,
  required AuthoringRevision3QuestDraftRequestV3 request,
  required Map<String, Object?> input,
}) {
  final generatorVersion = request.intent.additionalObjectiveTitles.isEmpty
      ? 2
      : 3;
  final source = revision3QuestGeneratedSource(
    technicalId: request.intent.technicalId,
    textHelper: request.intent.textHelper,
    parentRuntimeClass: 'UQuest_SwampCamp_SCChapter2',
    giverRuntimeUniqueName: 'OM_GRD_Asghan_263',
    title: request.intent.title,
    description: request.intent.description,
    objectiveTitle: request.intent.objectiveTitle,
    additionalObjectiveTitles: request.intent.additionalObjectiveTitles,
  );
  return <String, Object?>{
    'id': request.scriptModuleId,
    'display_name': '${request.displayName} Script',
    'origin': <String, Object?>{
      'type': 'generated',
      'generator_id': 'gore-authoring.draft-quest-skeleton',
      'generator_version': generatorVersion,
      'owner': <String, Object?>{
        'project_id': projectId,
        'id': request.questId,
        'expected_kind': 'quest_draft',
      },
    },
    'revision': 0,
    'payload': <String, Object?>{
      'kind': 'script_module',
      'data': <String, Object?>{
        'generator_id': 'gore-authoring.draft-quest-skeleton',
        'generator_version': generatorVersion,
        'owner': <String, Object?>{
          'project_id': projectId,
          'id': request.questId,
          'expected_kind': 'quest_draft',
        },
        'module_namespace': request.intent.moduleNamespace,
        'module_relative_path':
            '${request.intent.moduleNamespace.replaceAll('.', '/')}.as',
        'source': source,
        'source_sha256': crypto.sha256.convert(utf8.encode(source)).toString(),
        'input_fingerprint': _questInputFingerprint(input),
        'status': <String, Object?>{
          'authoring': 'offline_draft',
          'runtime': 'runtime_unqualified',
        },
      },
    },
  };
}

Map<String, Object?> _questPreparedResponse({
  required AuthoringWorkingHead basisHead,
  required AuthoringWorkingHead candidateHead,
  required String candidateProjectJson,
  required int revision,
  required String questId,
  required String scriptModuleId,
}) => <String, Object?>{
  'ok': true,
  'outcome': 'prepared_unpublished',
  'basis_head_json': basisHead.canonicalJson,
  'head_json': candidateHead.canonicalJson,
  'project_json': candidateProjectJson,
  'revision': revision,
  'quest_id': questId,
  'script_module_id': scriptModuleId,
  'artifact_deduplicated': false,
  'build_status': 'blocked',
  'runtime_status': 'runtime_unqualified',
  'artifact_authority': 'not_granted',
  'source_inspection': 'fresh_capability_required',
  'publication_status': 'not_supported',
};

AuthoringRevision3QuestDraftIntentV3 _questIntent(
  int ordinal, {
  List<String> additionalObjectiveTitles = const <String>[],
}) => AuthoringRevision3QuestDraftIntentV3(
  moduleNamespace: 'GoreMods.Quests.Managed$ordinal',
  technicalId: 'GORE_MANAGED_QUEST_$ordinal',
  textHelper: 'GoreManagedQuest${ordinal}Text',
  parentCatalogId: 'g1r:quest-parent:swampcamp_scchapter2',
  giverCatalogId: 'g1r:npc:om_grd_asghan_263',
  title: 'Managed Quest $ordinal',
  description: 'Exercise safe managed Quest publication.',
  objectiveTitle: 'Finish Managed Quest $ordinal',
  additionalObjectiveTitles: additionalObjectiveTitles,
);

AuthoringRevision3NpcDraftIntentV1 _npcIntent(int ordinal) =>
    AuthoringRevision3NpcDraftIntentV1(
      moduleNamespace: 'GoreMods.Npcs.Managed$ordinal',
      uniqueName: 'GoreManagedNpc$ordinal',
      parentCatalogId: 'g1r:npc:om_grd_asghan_263',
    );

Map<String, Object?> _preparedResponse(AuthoringWorkingHead head) =>
    <String, Object?>{'ok': true, 'head_json': head.canonicalJson};

Map<String, Object?> _openedResponse(
  AuthoringWorkingHead head,
  String projectJson,
) => <String, Object?>{..._preparedResponse(head), 'project_json': projectJson};
