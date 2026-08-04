import 'dart:collection';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/core_service.dart';
import 'package:gore_mod/core/mod_ffi.dart';

import '_revision3_voice_batch_test_support.dart';

void main() {
  test(
    'Voice batch plan request preserves the frozen wire-key order',
    () async {
      final fixture = Revision3VoiceBatchTestFixture.create();
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_plan_revision3_voice_batch_v1': fixture
              .planResponse(),
        },
      );

      final result = await ModFfi(core).authoringStorePlanRevision3VoiceBatchV1(
        root: r'C:\Mods\Asghan',
        gameRoot: voiceBatchTestGameRoot,
        sourceFolder: voiceBatchTestSourceFolder,
        locale: voiceBatchTestLocale,
        currentProjectJson: fixture.projectJson,
        expectedHead: fixture.basisHead,
      );

      expect(result.canPrepare, isTrue);
      expect(core.calls, hasLength(1));
      expect(
        core.calls.single.payload.keys.toList(growable: false),
        const <String>[
          'current_project_json',
          'expected_head_json',
          'game_root',
          'locale',
          'root',
          'source_folder',
        ],
      );
      expect(core.calls.single.payload, <String, Object?>{
        'current_project_json': fixture.projectJson,
        'expected_head_json': fixture.basisHead.canonicalJson,
        'game_root': voiceBatchTestGameRoot,
        'locale': voiceBatchTestLocale,
        'root': r'C:\Mods\Asghan',
        'source_folder': voiceBatchTestSourceFolder,
      });
    },
  );

  test(
    'Voice batch prepare request preserves source and plan seals in frozen order',
    () async {
      final fixture = Revision3VoiceBatchTestFixture.create();
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_prepare_revision3_voice_batch_v1': fixture
              .preparationResponse(),
        },
      );

      final prepared = await ModFfi(core)
          .authoringStorePrepareRevision3VoiceBatchV1(
            root: r'C:\Mods\Asghan',
            gameRoot: voiceBatchTestGameRoot,
            sourceFolder: voiceBatchTestSourceFolder,
            currentProjectJson: fixture.projectJson,
            plan: fixture.plan(),
          );

      expect(prepared.revision, 8);
      expect(core.calls, hasLength(1));
      expect(
        core.calls.single.payload.keys.toList(growable: false),
        const <String>[
          'current_project_json',
          'expected_head_json',
          'game_root',
          'locale',
          'root',
          'source_folder',
          'expected_source_manifest_sha256',
          'expected_plan_sha256',
        ],
      );
      expect(
        core.calls.single.payload['expected_source_manifest_sha256'],
        voiceBatchTestManifestSha,
      );
      expect(
        core.calls.single.payload['expected_plan_sha256'],
        voiceBatchTestPlanSha,
      );
    },
  );

  test(
    'response object order is irrelevant but the exact key set is closed',
    () {
      final fixture = Revision3VoiceBatchTestFixture.create();
      final response = fixture.planResponse();
      final reordered = LinkedHashMap<String, Object?>.fromEntries(
        response.entries.toList(growable: false).reversed,
      );
      final item = (response['items']! as List<Object?>).single!;
      final itemMap = (item as Map).cast<String, Object?>();
      reordered['items'] = <Object?>[
        LinkedHashMap<String, Object?>.fromEntries(
          itemMap.entries.toList(growable: false).reversed,
        ),
      ];

      expect(
        AuthoringRevision3VoiceBatchPlanResult.fromJson(
          reordered,
          expectedHead: fixture.basisHead,
          currentProjectJson: fixture.projectJson,
          expectedLocale: voiceBatchTestLocale,
        ).readyCount,
        1,
      );

      reordered['invented_authority'] = true;
      expect(
        () => AuthoringRevision3VoiceBatchPlanResult.fromJson(
          reordered,
          expectedHead: fixture.basisHead,
          currentProjectJson: fixture.projectJson,
          expectedLocale: voiceBatchTestLocale,
        ),
        throwsFormatException,
      );
    },
  );

  test('ASCII-fold collisions require deterministic byte ordering', () {
    final fixture = Revision3VoiceBatchTestFixture.create();

    final accepted = AuthoringRevision3VoiceBatchPlanResult.fromJson(
      voiceBatchCollisionPlanResponse(fixture: fixture),
      expectedHead: fixture.basisHead,
      currentProjectJson: fixture.projectJson,
      expectedLocale: voiceBatchTestLocale,
    );
    expect(accepted.items.map((item) => item.sourceName), const <String>[
      'LINE.ogg',
      'line.ogg',
    ]);
    expect(accepted.blockedCount, 2);

    expect(
      () => AuthoringRevision3VoiceBatchPlanResult.fromJson(
        voiceBatchCollisionPlanResponse(fixture: fixture, reverse: true),
        expectedHead: fixture.basisHead,
        currentProjectJson: fixture.projectJson,
        expectedLocale: voiceBatchTestLocale,
      ),
      throwsFormatException,
    );
  });

  test(
    'plan parser rejects partial facts, inconsistent counts, and authority',
    () {
      final fixture = Revision3VoiceBatchTestFixture.create();

      Map<String, Object?> mutatedItem(
        void Function(Map<String, Object?> item) mutate,
      ) {
        final response = voiceBatchDeepCopy(fixture.planResponse());
        final item = ((response['items']! as List).single as Map)
            .cast<String, Object?>();
        mutate(item);
        return response;
      }

      final malformed = <Map<String, Object?>>[
        mutatedItem((item) => item['line_id'] = null),
        mutatedItem((item) => item['asset'] = null),
        voiceBatchDeepCopy(fixture.planResponse())
          ..['ready_count'] = 0
          ..['blocked_count'] = 1,
        voiceBatchDeepCopy(fixture.planResponse())..['scanned_entry_count'] = 2,
        voiceBatchDeepCopy(fixture.planResponse())..['build_status'] = 'ready',
        voiceBatchDeepCopy(fixture.planResponse())
          ..['target_authority'] = 'granted',
        voiceBatchDeepCopy(fixture.planResponse())
          ..['publication_status'] = 'published',
      ];

      for (final response in malformed) {
        expect(
          () => AuthoringRevision3VoiceBatchPlanResult.fromJson(
            response,
            expectedHead: fixture.basisHead,
            currentProjectJson: fixture.projectJson,
            expectedLocale: voiceBatchTestLocale,
          ),
          throwsFormatException,
          reason: response.toString(),
        );
      }
    },
  );

  test(
    'prepare parser rejects partial items, non-exact candidates, and authority',
    () {
      final fixture = Revision3VoiceBatchTestFixture.create();
      final plan = fixture.plan();

      Map<String, Object?> mutatedItem(
        void Function(Map<String, Object?> item) mutate,
      ) {
        final response = voiceBatchDeepCopy(fixture.preparationResponse());
        final item = ((response['items']! as List).single as Map)
            .cast<String, Object?>();
        mutate(item);
        return response;
      }

      final nonExactCandidate = voiceBatchDeepCopy(
        fixture.preparationResponse(),
      );
      final project = (nonExactCandidate['project_json']! as String)
          .replaceFirst('Asghan German recording', 'Invented display name');
      nonExactCandidate['project_json'] = project;

      final malformed = <Map<String, Object?>>[
        mutatedItem((item) => item['selected'] = true),
        mutatedItem((item) => item['ogg'] = null),
        voiceBatchDeepCopy(fixture.preparationResponse())
          ..['imported_count'] = 2,
        voiceBatchDeepCopy(fixture.preparationResponse())
          ..['runtime_status'] = 'runtime_qualified',
        voiceBatchDeepCopy(fixture.preparationResponse())
          ..['target_authority'] = 'granted',
        nonExactCandidate,
      ];

      for (final response in malformed) {
        expect(
          () => AuthoringRevision3VoiceBatchPreparation.fromJson(
            response,
            currentProjectJson: fixture.projectJson,
            plan: plan,
          ),
          throwsFormatException,
          reason: response.toString(),
        );
      }
    },
  );

  test('prepare reconstructs the exact existing-slot delta', () {
    final fixture = Revision3VoiceBatchTestFixture.create(existingSlot: true);
    final plan = fixture.plan();

    final prepared = fixture.preparation(forPlan: plan);

    expect(plan.items.single.slotCreated, isFalse);
    expect(plan.items.single.slotId, voiceBatchTestExistingSlotId);
    expect(prepared.items.single.slotCreated, isFalse);
    expect(prepared.projectJson, fixture.candidateProjectJson);
    expect(prepared.revision, 8);
  });

  test(
    'ModFfi wraps malformed batch authority as a closed native failure',
    () async {
      final fixture = Revision3VoiceBatchTestFixture.create();
      final response = fixture.planResponse()..['build_status'] = 'ready';
      final core = FakeGoreCoreFfiService(
        responses: <String, Map<String, Object?>>{
          'authoring_store_plan_revision3_voice_batch_v1': response,
        },
      );

      await expectLater(
        ModFfi(core).authoringStorePlanRevision3VoiceBatchV1(
          root: r'C:\Mods\Asghan',
          gameRoot: voiceBatchTestGameRoot,
          sourceFolder: voiceBatchTestSourceFolder,
          locale: voiceBatchTestLocale,
          currentProjectJson: fixture.projectJson,
          expectedHead: fixture.basisHead,
        ),
        throwsA(
          isA<ModFfiException>()
              .having(
                (error) => error.command,
                'command',
                'authoring_store_plan_revision3_voice_batch_v1',
              )
              .having(
                (error) => error.code,
                'code',
                ModFfiException.malformedNativeResponseCode,
              ),
        ),
      );
    },
  );

  test('deduplication evidence stays a bool within an exact prepared item', () {
    final fixture = Revision3VoiceBatchTestFixture.create();
    final response = voiceBatchDeepCopy(fixture.preparationResponse());
    final item = ((response['items']! as List).single as Map)
        .cast<String, Object?>();
    item['asset_deduplicated'] = 'yes';

    expect(
      () => AuthoringRevision3VoiceBatchPreparation.fromJson(
        response,
        currentProjectJson: fixture.projectJson,
        plan: fixture.plan(),
      ),
      throwsFormatException,
    );
  });
}
