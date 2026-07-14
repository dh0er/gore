import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/project_controller.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dataasset_authoring.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';
import 'package:gore_mod/project/revision3_quest_context_authoring.dart';
import 'package:gore_mod/project/revision3_quest_outline_authoring.dart';
import 'package:gore_mod/project/revision3_quest_transitions_authoring.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';
import 'package:gore_mod/project/revision3_voice_take_selection_authoring.dart';

import '../support/revision3_dataasset_fixture.dart';
import '../support/revision3_npc_fixture.dart';
import '../support/revision3_voice_content_fixture.dart';
import '../support/revision3_voice_fixture.dart';
import '../support/revision3_quest_outline_fixture.dart';
import '../dataasset/dataasset_test_fixtures.dart';

void main() {
  test(
    'failed managed open preserves the exact legacy current project',
    () async {
      final legacy = _FakeLegacyLease(
        path: 'current.goremod',
        hasUnsavedChanges: true,
      );
      final expectedError = StateError('candidate could not be opened');
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async => throw expectedError,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final before = coordinator.state;

      await expectLater(
        coordinator.openManagedRevision3(Directory('candidate')),
        throwsA(same(expectedError)),
      );

      expect(coordinator.state, same(before));
      expect(coordinator.state, isA<LegacyCurrentProjectState>());
      expect(legacy.closeCalls, 0);
      expect(legacy.saveCalls, 0);
    },
  );

  test(
    'managed adoption closes legacy and save verifies without a managed write',
    () async {
      final legacy = _FakeLegacyLease(path: 'old.goremod');
      final managed = _FakeManagedLease(
        root: Directory('managed'),
        projectIdValue: '01010101010101010101010101010101',
        projectRevision: 7,
        head: _head(7),
      );
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });

      final opened = await coordinator.openManagedRevision3(
        Directory('caller-path'),
      );
      expect(opened.root.path, managed.root.path);
      expect(opened.projectId, managed.projectId);
      expect(opened.projectRevision, 7);
      expect(opened.head.canonicalJson, managed.head.canonicalJson);
      expect(legacy.closeCalls, 1);
      expect(managed.verifyCalls, 0);

      final saved = await coordinator.saveCurrent();
      expect(saved, isA<ManagedRevision3CurrentProjectState>());
      expect(managed.verifyCalls, 1);
      expect(managed.closeCalls, 0);

      final verified = await coordinator.verifyCurrent();
      expect(verified.projectRevision, 7);
      expect(managed.verifyCalls, 2);
    },
  );

  test('save and managed open share one invocation-ordered lane', () async {
    final events = <String>[];
    final saveEntered = Completer<void>();
    final releaseSave = Completer<void>();
    final legacy = _FakeLegacyLease(
      path: 'ordered.goremod',
      onSave: () async {
        events.add('save-enter');
        saveEntered.complete();
        await releaseSave.future;
        events.add('save-exit');
      },
      onClose: () => events.add('legacy-close'),
    );
    final managed = _FakeManagedLease(
      root: Directory('ordered-managed'),
      projectIdValue: '02020202020202020202020202020202',
      projectRevision: 2,
      head: _head(2),
    );
    var openCalls = 0;
    final coordinator = CurrentProjectCoordinator(
      initialLegacy: legacy,
      openManagedRevision3: (_) async {
        openCalls++;
        events.add('managed-open');
        return managed;
      },
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });

    final save = coordinator.saveCurrent();
    await saveEntered.future;
    final open = coordinator.openManagedRevision3(Directory('managed'));
    await Future<void>.delayed(Duration.zero);
    expect(openCalls, 0);

    releaseSave.complete();
    await save;
    await open;
    expect(events, <String>[
      'save-enter',
      'save-exit',
      'managed-open',
      'legacy-close',
    ]);
  });

  test(
    'invalid managed candidate snapshot is closed and legacy remains current',
    () async {
      final legacy = _FakeLegacyLease(path: 'preserved.goremod');
      final candidate = _FakeManagedLease(
        root: Directory('invalid'),
        projectIdValue: '03030303030303030303030303030303',
        projectRevision: 3,
        head: _head(3),
        projectIdError: StateError('invalid candidate identity'),
      );
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async => candidate,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });

      await expectLater(
        coordinator.openManagedRevision3(Directory('invalid')),
        throwsA(isA<StateError>()),
      );

      expect(coordinator.state, isA<LegacyCurrentProjectState>());
      expect(legacy.closeCalls, 0);
      expect(candidate.closeCalls, 1);
    },
  );

  test('managed verification failure refreshes visible reopen state', () async {
    final managed = _FakeManagedLease(
      root: Directory('poisoned'),
      projectIdValue: '04040404040404040404040404040404',
      projectRevision: 4,
      head: _head(4),
      onVerify: (lease) {
        lease.requiresReopenValue = true;
        throw StateError('full reopen failed');
      },
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(Directory('poisoned'));

    await expectLater(coordinator.saveCurrent(), throwsA(isA<StateError>()));

    final state = coordinator.state as ManagedRevision3CurrentProjectState;
    expect(state.requiresReopen, isTrue);
    expect(managed.verifyCalls, 1);

    await expectLater(
      coordinator.saveCurrent(),
      throwsA(isA<CurrentProjectOperationUnsupportedException>()),
    );
    await expectLater(
      coordinator.verifyCurrent(),
      throwsA(isA<CurrentProjectOperationUnsupportedException>()),
    );
    expect(managed.verifyCalls, 1);
  });

  test('adopting legacy retires the managed lease exactly once', () async {
    final managed = _FakeManagedLease(
      root: Directory('before-legacy'),
      projectIdValue: '05050505050505050505050505050505',
      projectRevision: 5,
      head: _head(5),
    );
    final legacy = _FakeLegacyLease(
      path: 'adopted.goremod',
      hasUnsavedChanges: false,
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(Directory('before-legacy'));

    final state = await coordinator.adoptLegacy(legacy);

    expect(state.path, 'adopted.goremod');
    expect(coordinator.state, isA<LegacyCurrentProjectState>());
    expect(managed.closeCalls, 1);
    expect(legacy.closeCalls, 0);
    await coordinator.saveCurrent();
    expect(legacy.saveCalls, 1);
    await expectLater(
      coordinator.verifyCurrent(),
      throwsA(isA<CurrentProjectOperationUnsupportedException>()),
    );
  });

  test(
    'production legacy factory creates one-shot leases across a format round trip',
    () async {
      final container = ProviderContainer.test();
      final factory = container.read(legacyCurrentProjectLeaseFactoryProvider);
      final firstLegacy = factory();
      final firstManaged = _FakeManagedLease(
        root: Directory('first-managed'),
        projectIdValue: '08080808080808080808080808080808',
        projectRevision: 8,
        head: _head(8),
      );
      final secondManaged = _FakeManagedLease(
        root: Directory('second-managed'),
        projectIdValue: '09090909090909090909090909090909',
        projectRevision: 9,
        head: _head(9),
      );
      final managed = <_FakeManagedLease>[firstManaged, secondManaged];
      var nextManaged = 0;
      container.read(currentProjectPathProvider.notifier).state =
          'first.goremod';
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: firstLegacy,
        openManagedRevision3: (_) async => managed[nextManaged++],
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
        container.dispose();
      });

      await coordinator.openManagedRevision3(Directory('first-managed'));
      expect(container.read(currentProjectPathProvider), isNull);

      container.read(currentProjectPathProvider.notifier).state =
          'second.goremod';
      final secondLegacy = factory();
      expect(secondLegacy, isNot(same(firstLegacy)));
      final legacyState = await coordinator.adoptLegacy(secondLegacy);
      expect(legacyState.path, 'second.goremod');
      expect(firstManaged.closeCalls, 1);

      await coordinator.openManagedRevision3(Directory('second-managed'));
      expect(container.read(currentProjectPathProvider), isNull);
      await expectLater(
        coordinator.adoptLegacy(firstLegacy),
        throwsA(isA<StateError>()),
      );
      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
    },
  );

  test(
    'close failure is terminal diagnostic state and is never retried',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('terminal-close'),
        projectIdValue: '06060606060606060606060606060606',
        projectRevision: 6,
        head: _head(6),
        closeFailuresRemaining: 1,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(coordinator.dispose);
      await coordinator.openManagedRevision3(Directory('terminal-close'));

      await expectLater(coordinator.closeCurrent(), throwsA(isA<StateError>()));
      expect(coordinator.state, isA<NoCurrentProjectState>());
      expect(coordinator.terminalCleanupFailures, hasLength(1));
      expect(
        coordinator.terminalCleanupFailures.single.projectKind,
        CurrentProjectKind.managedRevision3,
      );
      expect(
        coordinator.terminalCleanupFailures.single.error,
        isA<StateError>(),
      );
      expect(managed.closeCalls, 1);

      await coordinator.shutdown();
      expect(managed.closeCalls, 1);
      expect(coordinator.terminalCleanupFailures, hasLength(1));
      await coordinator.shutdown();
      expect(managed.closeCalls, 1);
    },
  );

  test(
    'retired close failure does not fail adoption or retry during shutdown',
    () async {
      final legacy = _FakeLegacyLease(
        path: 'terminal-retired.goremod',
        onClose: () => throw StateError('injected retired close failure'),
      );
      final managed = _FakeManagedLease(
        root: Directory('after-terminal-retire'),
        projectIdValue: '0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a',
        projectRevision: 10,
        head: _head(10),
      );
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(coordinator.dispose);

      final opened = await coordinator.openManagedRevision3(
        Directory('after-terminal-retire'),
      );
      expect(opened.projectRevision, 10);
      expect(legacy.closeCalls, 1);
      expect(coordinator.terminalCleanupFailures, hasLength(1));
      expect(
        coordinator.terminalCleanupFailures.single.projectKind,
        CurrentProjectKind.legacyFormat1,
      );

      await coordinator.shutdown();
      expect(legacy.closeCalls, 1);
      expect(managed.closeCalls, 1);
      expect(coordinator.terminalCleanupFailures, hasLength(1));
    },
  );

  test('shutdown drains an accepted late open and closes its result', () async {
    final openEntered = Completer<void>();
    final releaseOpen = Completer<void>();
    final legacy = _FakeLegacyLease(path: 'initial.goremod');
    final managed = _FakeManagedLease(
      root: Directory('late'),
      projectIdValue: '07070707070707070707070707070707',
      projectRevision: 7,
      head: _head(7),
    );
    final coordinator = CurrentProjectCoordinator(
      initialLegacy: legacy,
      openManagedRevision3: (_) async {
        openEntered.complete();
        await releaseOpen.future;
        return managed;
      },
    );
    addTearDown(coordinator.dispose);

    final opening = coordinator.openManagedRevision3(Directory('late'));
    await openEntered.future;
    final shutdown = coordinator.shutdown();
    await expectLater(
      coordinator.saveCurrent(),
      throwsA(isA<CurrentProjectCoordinatorClosedException>()),
    );
    releaseOpen.complete();
    await opening;
    await shutdown;

    expect(legacy.closeCalls, 1);
    expect(managed.closeCalls, 1);
    expect(coordinator.state, isA<NoCurrentProjectState>());
  });

  test(
    'dispose drains an accepted save without reading disposed state',
    () async {
      final saveEntered = Completer<void>();
      final releaseSave = Completer<void>();
      final legacy = _FakeLegacyLease(
        path: 'dispose.goremod',
        onSave: () async {
          saveEntered.complete();
          await releaseSave.future;
        },
      );
      final coordinator = CurrentProjectCoordinator(
        initialLegacy: legacy,
        openManagedRevision3: (_) async => throw UnimplementedError(),
      );

      final saving = coordinator.saveCurrent();
      await saveEntered.future;
      coordinator.dispose();
      releaseSave.complete();

      final saved = await saving;
      expect(saved, isA<LegacyCurrentProjectState>());
      await coordinator.shutdown();
      expect(legacy.closeCalls, 1);
    },
  );

  test(
    'legacy open is validated in the coordinator lane before replacing managed',
    () async {
      final candidate = _FakeLegacyLease();
      final managed = _FakeManagedLease(
        root: Directory('managed-before-legacy-open'),
        projectIdValue: '11111111111111111111111111111111',
        projectRevision: 11,
        head: _head(11),
      );
      final coordinator = CurrentProjectCoordinator(
        createLegacy: () => candidate,
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final opened = await coordinator.openLegacyFromPath('opened.goremod');

      expect(opened.path, 'opened.goremod');
      expect(coordinator.state, isA<LegacyCurrentProjectState>());
      expect(candidate.openFromPathCalls, 1);
      expect(candidate.closeCalls, 0);
      expect(managed.closeCalls, 1);
    },
  );

  test(
    'failed legacy candidate open preserves the exact managed current project',
    () async {
      final expectedError = StateError('legacy candidate is invalid');
      final candidate = _FakeLegacyLease(
        onOpenFromPath: (_) => throw expectedError,
      );
      final managed = _FakeManagedLease(
        root: Directory('managed-preserved'),
        projectIdValue: '12121212121212121212121212121212',
        projectRevision: 12,
        head: _head(12),
      );
      final coordinator = CurrentProjectCoordinator(
        createLegacy: () => candidate,
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);
      final before = coordinator.state;

      await expectLater(
        coordinator.openLegacyFromPath('invalid.goremod'),
        throwsA(same(expectedError)),
      );

      expect(coordinator.state, same(before));
      expect(candidate.openFromPathCalls, 1);
      expect(candidate.closeCalls, 1);
      expect(managed.closeCalls, 0);
    },
  );

  test('managed current project rejects compatibility Save As', () async {
    final managed = _FakeManagedLease(
      root: Directory('managed-no-save-as'),
      projectIdValue: '13131313131313131313131313131313',
      projectRevision: 13,
      head: _head(13),
    );
    final coordinator = CurrentProjectCoordinator(
      createLegacy: () => _FakeLegacyLease(),
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.saveLegacyToPath('forbidden.goremod'),
      throwsA(isA<CurrentProjectOperationUnsupportedException>()),
    );

    expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());
    expect(managed.verifyCalls, 0);
  });

  test(
    'managed content read uses the current lease and refreshes poison state',
    () async {
      final projectId = '14141414141414141414141414141414';
      final index = _contentIndex(projectId: projectId, revision: 14);
      final managed = _FakeManagedLease(
        root: Directory('managed-content'),
        projectIdValue: projectId,
        projectRevision: 14,
        head: _head(14),
        contentIndex: index,
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      expect(await coordinator.readCurrentRevision3ContentIndex(), same(index));
      expect(managed.contentReadCalls, 1);
      expect(coordinator.state, isA<ManagedRevision3CurrentProjectState>());

      managed.onContentRead = (lease) {
        lease.requiresReopenValue = true;
        throw StateError('injected content verification failure');
      };
      await expectLater(
        coordinator.readCurrentRevision3ContentIndex(),
        throwsA(isA<Revision3ContentRequiresReopenException>()),
      );
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        coordinator.readCurrentRevision3ContentIndex(),
        throwsA(isA<Revision3ContentRequiresReopenException>()),
      );
      expect(managed.contentReadCalls, 2);
    },
  );

  test(
    'Quest source inspection binds the visible root, identity, revision, and head',
    () async {
      const projectId = '41414141414141414141414141414141';
      const questId = '71717171717171717171717171717171';
      const gameRoot = r'C:\Games\Gothic Remake';
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-inspection'),
        projectIdValue: projectId,
        projectRevision: 41,
        head: _head(41),
        onQuestInspection: (lease, receivedGameRoot, receivedQuestId) {
          expect(receivedGameRoot, gameRoot);
          expect(receivedQuestId, questId);
          return _controllerQuestInspectionResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            questId: receivedQuestId,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      final staleRequests =
          <Future<AuthoringRevision3QuestSourceInspectionResult> Function()>[
            () => coordinator.inspectCurrentRevision3QuestSource(
              expectedRoot: 'another-root',
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              gameRoot: gameRoot,
              questId: questId,
            ),
            () => coordinator.inspectCurrentRevision3QuestSource(
              expectedRoot: visible.root.path,
              expectedProjectId: '42424242424242424242424242424242',
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              gameRoot: gameRoot,
              questId: questId,
            ),
            () => coordinator.inspectCurrentRevision3QuestSource(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision + 1,
              expectedHead: visible.head,
              gameRoot: gameRoot,
              questId: questId,
            ),
            () => coordinator.inspectCurrentRevision3QuestSource(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: _head(42),
              gameRoot: gameRoot,
              questId: questId,
            ),
          ];
      for (final inspect in staleRequests) {
        await expectLater(
          inspect(),
          throwsA(
            isA<Revision3QuestSourceInspectionStaleCheckpointException>(),
          ),
        );
      }
      expect(managed.questInspectionCalls, 0);

      final result = await coordinator.inspectCurrentRevision3QuestSource(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        gameRoot: gameRoot,
        questId: questId,
      );

      expect(result.projectId, visible.projectId);
      expect(result.projectRevision, visible.projectRevision);
      expect(result.head.canonicalJson, visible.head.canonicalJson);
      expect(result.questId, questId);
      expect(managed.questInspectionCalls, 1);
      expect(managed.questInspectionGameRoots, <String>[gameRoot]);
      expect(managed.questInspectionQuestIds, <String>[questId]);
      final after = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(after.root.path, visible.root.path);
      expect(after.projectId, visible.projectId);
      expect(after.projectRevision, visible.projectRevision);
      expect(after.head.canonicalJson, visible.head.canonicalJson);
      expect(after.requiresReopen, isFalse);
    },
  );

  test('Quest source inspection maps a mismatched result to stale', () async {
    const projectId = '43434343434343434343434343434343';
    const questId = '73737373737373737373737373737373';
    final managed = _FakeManagedLease(
      root: Directory('managed-quest-inspection-result-stale'),
      projectIdValue: projectId,
      projectRevision: 43,
      head: _head(43),
      onQuestInspection: (lease, _, receivedQuestId) =>
          _controllerQuestInspectionResult(
            head: _head(44),
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            questId: receivedQuestId,
          ),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.inspectCurrentRevision3QuestSource(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        gameRoot: r'C:\Games\Gothic Remake',
        questId: questId,
      ),
      throwsA(isA<Revision3QuestSourceInspectionStaleCheckpointException>()),
    );

    expect(managed.questInspectionCalls, 1);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isFalse,
    );
  });

  test(
    'Quest source inspection maps poisoned lease state to requires-reopen and locks retry',
    () async {
      const projectId = '45454545454545454545454545454545';
      const questId = '75757575757575757575757575757575';
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-inspection-reopen'),
        projectIdValue: projectId,
        projectRevision: 45,
        head: _head(45),
        onQuestInspection: (lease, _, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected Quest inspection integrity failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      Future<AuthoringRevision3QuestSourceInspectionResult> inspect() =>
          coordinator.inspectCurrentRevision3QuestSource(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            gameRoot: r'C:\Games\Gothic Remake',
            questId: questId,
          );

      await expectLater(
        inspect(),
        throwsA(isA<Revision3QuestSourceInspectionRequiresReopenException>()),
      );
      expect(managed.questInspectionCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        inspect(),
        throwsA(isA<Revision3QuestSourceInspectionRequiresReopenException>()),
      );
      expect(managed.questInspectionCalls, 1);
    },
  );

  test(
    'NPC source inspection binds the exact visible checkpoint without a game root',
    () async {
      const projectId = '46464646464646464646464646464646';
      const npcId = '76767676767676767676767676767676';
      final managed = _FakeManagedLease(
        root: Directory('managed-npc-inspection'),
        projectIdValue: projectId,
        projectRevision: 46,
        head: _head(46),
        onNpcInspection: (lease, receivedNpcId) {
          expect(receivedNpcId, npcId);
          return revision3NpcInspectionResult(
            head: lease.head,
            projectJson: revision3NpcInspectionProjectJson(
              projectId: lease.projectId,
              revision: lease.projectRevision,
            ),
            npcId: receivedNpcId,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      final staleRequests =
          <Future<AuthoringRevision3NpcSourceInspectionResult> Function()>[
            () => coordinator.inspectCurrentRevision3NpcSource(
              expectedRoot: 'another-root',
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              npcId: npcId,
            ),
            () => coordinator.inspectCurrentRevision3NpcSource(
              expectedRoot: visible.root.path,
              expectedProjectId: '47474747474747474747474747474747',
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              npcId: npcId,
            ),
            () => coordinator.inspectCurrentRevision3NpcSource(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision + 1,
              expectedHead: visible.head,
              npcId: npcId,
            ),
            () => coordinator.inspectCurrentRevision3NpcSource(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: _head(47),
              npcId: npcId,
            ),
          ];
      for (final inspect in staleRequests) {
        await expectLater(
          inspect(),
          throwsA(isA<Revision3NpcSourceInspectionStaleCheckpointException>()),
        );
      }
      expect(managed.npcInspectionCalls, 0);

      final result = await coordinator.inspectCurrentRevision3NpcSource(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        npcId: npcId,
      );

      expect(result.projectId, visible.projectId);
      expect(result.projectRevision, visible.projectRevision);
      expect(result.head.canonicalJson, visible.head.canonicalJson);
      expect(result.npcId, npcId);
      expect(managed.npcInspectionCalls, 1);
      expect(managed.npcInspectionNpcIds, <String>[npcId]);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
    },
  );

  test('NPC source inspection maps a mismatched result to stale', () async {
    const projectId = '48484848484848484848484848484848';
    const npcId = '78787878787878787878787878787878';
    final managed = _FakeManagedLease(
      root: Directory('managed-npc-inspection-result-stale'),
      projectIdValue: projectId,
      projectRevision: 48,
      head: _head(48),
      onNpcInspection: (lease, receivedNpcId) => revision3NpcInspectionResult(
        head: _head(49),
        projectJson: revision3NpcInspectionProjectJson(
          projectId: lease.projectId,
          revision: lease.projectRevision,
        ),
        npcId: receivedNpcId,
      ),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.inspectCurrentRevision3NpcSource(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        npcId: npcId,
      ),
      throwsA(isA<Revision3NpcSourceInspectionStaleCheckpointException>()),
    );

    expect(managed.npcInspectionCalls, 1);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isFalse,
    );
  });

  test(
    'NPC source inspection maps poisoned lease state to requires-reopen and locks retry',
    () async {
      const projectId = '49494949494949494949494949494949';
      const npcId = '79797979797979797979797979797979';
      final managed = _FakeManagedLease(
        root: Directory('managed-npc-inspection-reopen'),
        projectIdValue: projectId,
        projectRevision: 49,
        head: _head(49),
        onNpcInspection: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected NPC inspection integrity failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      Future<AuthoringRevision3NpcSourceInspectionResult> inspect() =>
          coordinator.inspectCurrentRevision3NpcSource(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            npcId: npcId,
          );

      await expectLater(
        inspect(),
        throwsA(isA<Revision3NpcSourceInspectionRequiresReopenException>()),
      );
      expect(managed.npcInspectionCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        inspect(),
        throwsA(isA<Revision3NpcSourceInspectionRequiresReopenException>()),
      );
      expect(managed.npcInspectionCalls, 1);
    },
  );

  test(
    'DataAsset package browsing binds the exact visible checkpoint and game root',
    () async {
      const projectId = '50505050505050505050505050505050';
      final managed = _FakeManagedLease(
        root: Directory('managed-dataasset-package-index'),
        projectIdValue: projectId,
        projectRevision: 50,
        head: _head(50),
        onDataAssetPackageIndexRead: (lease, gameRoot) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          return _controllerDataAssetPackageIndexResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      for (final read
          in <Future<AuthoringRevision3DataAssetPackageIndexResult> Function()>[
            () => coordinator.readCurrentRevision3DataAssetPackageIndex(
              expectedRoot: 'another-root',
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              gameRoot: r'C:\Games\Gothic Remake',
            ),
            () => coordinator.readCurrentRevision3DataAssetPackageIndex(
              expectedRoot: visible.root.path,
              expectedProjectId: '51515151515151515151515151515151',
              expectedProjectRevision: visible.projectRevision,
              expectedHead: visible.head,
              gameRoot: r'C:\Games\Gothic Remake',
            ),
            () => coordinator.readCurrentRevision3DataAssetPackageIndex(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision + 1,
              expectedHead: visible.head,
              gameRoot: r'C:\Games\Gothic Remake',
            ),
            () => coordinator.readCurrentRevision3DataAssetPackageIndex(
              expectedRoot: visible.root.path,
              expectedProjectId: visible.projectId,
              expectedProjectRevision: visible.projectRevision,
              expectedHead: _head(51),
              gameRoot: r'C:\Games\Gothic Remake',
            ),
          ]) {
        await expectLater(
          read(),
          throwsA(
            isA<Revision3DataAssetPackageIndexStaleCheckpointException>(),
          ),
        );
      }
      await expectLater(
        coordinator.readCurrentRevision3DataAssetPackageIndex(
          expectedRoot: visible.root.path,
          expectedProjectId: visible.projectId,
          expectedProjectRevision: visible.projectRevision,
          expectedHead: visible.head,
          gameRoot: '',
        ),
        throwsA(isA<CurrentProjectOperationUnsupportedException>()),
      );
      expect(managed.dataAssetPackageIndexReadCalls, 0);

      final result = await coordinator
          .readCurrentRevision3DataAssetPackageIndex(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            gameRoot: r'C:\Games\Gothic Remake',
          );

      expect(result.projectId, visible.projectId);
      expect(result.projectRevision, visible.projectRevision);
      expect(
        result.index.candidates.single.targetPath,
        '/Game/Characters/DA_Asghan',
      );
      expect(managed.dataAssetPackageIndexReadCalls, 1);
      expect(managed.dataAssetPackageIndexGameRoots, <String>[
        r'C:\Games\Gothic Remake',
      ]);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isFalse,
      );
    },
  );

  test('DataAsset package result mismatch maps to stale', () async {
    const projectId = '52525252525252525252525252525252';
    final managed = _FakeManagedLease(
      root: Directory('managed-dataasset-package-index-stale'),
      projectIdValue: projectId,
      projectRevision: 52,
      head: _head(52),
      onDataAssetPackageIndexRead: (lease, _) =>
          _controllerDataAssetPackageIndexResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision + 1,
          ),
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    final visible = await coordinator.openManagedRevision3(managed.root);

    await expectLater(
      coordinator.readCurrentRevision3DataAssetPackageIndex(
        expectedRoot: visible.root.path,
        expectedProjectId: visible.projectId,
        expectedProjectRevision: visible.projectRevision,
        expectedHead: visible.head,
        gameRoot: r'C:\Games\Gothic Remake',
      ),
      throwsA(isA<Revision3DataAssetPackageIndexStaleCheckpointException>()),
    );
    expect(managed.dataAssetPackageIndexReadCalls, 1);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isFalse,
    );
  });

  test(
    'DataAsset package browsing maps poisoned lease state to requires-reopen',
    () async {
      const projectId = '53535353535353535353535353535353';
      final managed = _FakeManagedLease(
        root: Directory('managed-dataasset-package-index-reopen'),
        projectIdValue: projectId,
        projectRevision: 53,
        head: _head(53),
        onDataAssetPackageIndexRead: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected DataAsset package-index failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      final visible = await coordinator.openManagedRevision3(managed.root);

      Future<AuthoringRevision3DataAssetPackageIndexResult> read() =>
          coordinator.readCurrentRevision3DataAssetPackageIndex(
            expectedRoot: visible.root.path,
            expectedProjectId: visible.projectId,
            expectedProjectRevision: visible.projectRevision,
            expectedHead: visible.head,
            gameRoot: r'C:\Games\Gothic Remake',
          );

      await expectLater(
        read(),
        throwsA(isA<Revision3DataAssetPackageIndexRequiresReopenException>()),
      );
      expect(managed.dataAssetPackageIndexReadCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        read(),
        throwsA(isA<Revision3DataAssetPackageIndexRequiresReopenException>()),
      );
      expect(managed.dataAssetPackageIndexReadCalls, 1);
    },
  );

  test(
    'Quest publication is exact-basis bound and refreshes R3 state',
    () async {
      final projectId = '15151515151515151515151515151515';
      late Revision3QuestDraftAuthoringInput receivedInput;
      final managed = _FakeManagedLease(
        root: Directory('managed-quest'),
        projectIdValue: projectId,
        projectRevision: 15,
        head: _head(15),
        onQuestPublish: (lease, gameRoot, input) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          receivedInput = input;
          lease.projectRevision = 16;
          lease.head = _head(16);
          return Revision3QuestDraftPublication(
            projectId: projectId,
            projectRevision: 16,
            questId: '25252525252525252525252525252525',
            scriptModuleId: '35353535353535353535353535353535',
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final published = await coordinator.createCurrentRevision3QuestDraft(
        expectedRoot: managed.root.path,
        expectedProjectId: projectId,
        expectedProjectRevision: 15,
        expectedHead: _head(15),
        gameRoot: r'C:\Games\Gothic Remake',
        input: _questInput(),
      );

      expect(published.projectRevision, 16);
      expect(receivedInput.title, 'Find Homer');
      expect(managed.questPublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 16);
      expect(state.head.canonicalJson, _head(16).canonicalJson);
    },
  );

  test(
    'Quest publication rejects same-id/revision different-head wizard before touching the lease',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-stale-quest'),
        projectIdValue: '16161616161616161616161616161616',
        projectRevision: 16,
        head: _head(16),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.createCurrentRevision3QuestDraft(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 16,
          expectedHead: _head(15),
          gameRoot: r'C:\Games\Gothic Remake',
          input: _questInput(),
        ),
        throwsA(isA<Revision3QuestDraftStaleCheckpointException>()),
      );
      expect(managed.questPublishCalls, 0);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        16,
      );
    },
  );

  test(
    'poisoned Quest failure locks retries and refreshes requiresReopen',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-poisoned-quest'),
        projectIdValue: '17171717171717171717171717171717',
        projectRevision: 17,
        head: _head(17),
        onQuestPublish: (lease, _, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected publication verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      Future<void> publish() async {
        await coordinator.createCurrentRevision3QuestDraft(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 17,
          expectedHead: _head(17),
          gameRoot: r'C:\Games\Gothic Remake',
          input: _questInput(),
        );
      }

      await expectLater(
        publish(),
        throwsA(isA<Revision3QuestDraftRequiresReopenException>()),
      );
      expect(managed.questPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        publish(),
        throwsA(isA<Revision3QuestDraftRequiresReopenException>()),
      );
      expect(managed.questPublishCalls, 1);
    },
  );

  test(
    'Quest outline edit is exact-checkpoint bound and needs no game root',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final index = fixture.contentIndex();
      final input = Revision3QuestOutlineEditInput.forQuest(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
        displayName: 'Find Homer safely',
        title: 'Find Homer safely',
        objectiveTitles: const [
          'Inspect the old gate',
          'Ask Asghan about Homer',
          'Report to Diego',
        ],
      );
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-outline'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: 7,
        head: _head(7),
        onQuestOutlinePublish: (lease, received) {
          expect(received, same(input));
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3QuestOutlineEditPublication(
            projectId: revision3QuestOutlineProjectId,
            projectRevision: 8,
            questId: revision3QuestOutlineQuestId,
            moduleId: revision3QuestOutlineModuleId,
            questRevision: 5,
            moduleRevision: 6,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final publication = await coordinator.editCurrentRevision3QuestOutline(
        expectedRoot: managed.root.path,
        expectedProjectId: revision3QuestOutlineProjectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        input: input,
      );

      expect(publication.projectRevision, 8);
      expect(managed.questOutlinePublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        8,
      );
    },
  );

  test(
    'Quest outline stale/reopen guards make zero additional lease calls',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final index = fixture.contentIndex();
      final input = Revision3QuestOutlineEditInput.forQuest(
        index: index,
        quest: index.entityById(revision3QuestOutlineQuestId)!,
        displayName: 'Find Homer safely',
        title: 'Find Homer safely',
        objectiveTitles: const [
          'Inspect the old gate',
          'Ask Asghan about Homer',
          'Report to Diego',
        ],
      );
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-outline-guard'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: 7,
        head: _head(7),
        onQuestOutlinePublish: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected outline verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.editCurrentRevision3QuestOutline(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: 7,
          expectedHead: _head(6),
          input: input,
        ),
        throwsA(isA<Revision3QuestOutlineStaleCheckpointException>()),
      );
      expect(managed.questOutlinePublishCalls, 0);

      await expectLater(
        coordinator.editCurrentRevision3QuestOutline(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: 7,
          expectedHead: _head(7),
          input: input,
        ),
        throwsA(isA<Revision3QuestOutlineRequiresReopenException>()),
      );
      expect(managed.questOutlinePublishCalls, 1);
      await expectLater(
        coordinator.editCurrentRevision3QuestOutline(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: 7,
          expectedHead: _head(7),
          input: input,
        ),
        throwsA(isA<Revision3QuestOutlineRequiresReopenException>()),
      );
      expect(managed.questOutlinePublishCalls, 1);
    },
  );

  test(
    'Quest transitions seed and publication are exact-checkpoint guarded without a game root',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final seed = _questTransitionsSeed(fixture);
      final plan = await _questTransitionsPlan(fixture);
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-transitions'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        onQuestTransitionsSeed:
            (lease, questId, questRevision, moduleId, moduleRevision) {
              expect(questId, revision3QuestOutlineQuestId);
              expect(questRevision, fixture.questRevision);
              expect(moduleId, revision3QuestOutlineModuleId);
              expect(moduleRevision, fixture.moduleRevision);
              return seed;
            },
        onQuestTransitionsPublish: (lease, received) {
          expect(received, same(plan));
          lease.projectRevision = fixture.projectRevision + 1;
          lease.head = _head(fixture.projectRevision + 1);
          return Revision3QuestTransitionsEditPublication(
            projectId: revision3QuestOutlineProjectId,
            projectRevision: fixture.projectRevision + 1,
            questId: revision3QuestOutlineQuestId,
            moduleId: revision3QuestOutlineModuleId,
            questRevision: fixture.questRevision + 1,
            moduleRevision: fixture.moduleRevision + 1,
            transitionPlanSeal: plan.transitionPlan.contentSeal,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final loaded = await coordinator.readCurrentRevision3QuestTransitionsSeed(
        expectedRoot: managed.root.path,
        expectedProjectId: revision3QuestOutlineProjectId,
        expectedProjectRevision: fixture.projectRevision,
        expectedHead: _head(fixture.projectRevision),
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixture.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: fixture.moduleRevision,
      );
      expect(loaded.transitionPlanSeal.sha256, seed.transitionPlanSeal.sha256);
      expect(managed.questTransitionsSeedCalls, 1);

      final publication = await coordinator
          .editCurrentRevision3QuestTransitions(
            expectedRoot: managed.root.path,
            expectedProjectId: revision3QuestOutlineProjectId,
            expectedProjectRevision: fixture.projectRevision,
            expectedHead: _head(fixture.projectRevision),
            plan: plan,
          );
      expect(publication.projectRevision, fixture.projectRevision + 1);
      expect(
        publication.transitionPlanSeal.sha256,
        plan.transitionPlan.contentSeal.sha256,
      );
      expect(managed.questTransitionsPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        fixture.projectRevision + 1,
      );
    },
  );

  test(
    'Quest transitions stale and reopen guards make no extra lease calls',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final plan = await _questTransitionsPlan(fixture);
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-transitions-guards'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        onQuestTransitionsPublish: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected transitions verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.editCurrentRevision3QuestTransitions(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: fixture.projectRevision,
          expectedHead: _head(fixture.projectRevision - 1),
          plan: plan,
        ),
        throwsA(isA<Revision3QuestTransitionsStaleCheckpointException>()),
      );
      expect(managed.questTransitionsPublishCalls, 0);

      await expectLater(
        coordinator.editCurrentRevision3QuestTransitions(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: fixture.projectRevision,
          expectedHead: _head(fixture.projectRevision),
          plan: plan,
        ),
        throwsA(isA<Revision3QuestTransitionsRequiresReopenException>()),
      );
      expect(managed.questTransitionsPublishCalls, 1);
      await expectLater(
        coordinator.editCurrentRevision3QuestTransitions(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: fixture.projectRevision,
          expectedHead: _head(fixture.projectRevision),
          plan: plan,
        ),
        throwsA(isA<Revision3QuestTransitionsRequiresReopenException>()),
      );
      expect(managed.questTransitionsPublishCalls, 1);
    },
  );

  test(
    'Quest context seed and publication are exact-checkpoint guarded',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final seed = _questContextSeed(fixture);
      final plan = await _questContextPlan(fixture);
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-context'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        onQuestContextSeed:
            (
              lease,
              questId,
              questRevision,
              moduleId,
              moduleRevision,
              parentRuntime,
              giverRuntime,
            ) {
              expect(questId, revision3QuestOutlineQuestId);
              expect(questRevision, fixture.questRevision);
              expect(moduleId, revision3QuestOutlineModuleId);
              expect(moduleRevision, fixture.moduleRevision);
              expect(parentRuntime, 'UQuest_SwampCamp_SCChapter2');
              expect(giverRuntime, 'OM_GRD_Asghan_263');
              return seed;
            },
        onQuestContextPublish: (lease, gameRoot, received) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          expect(received, same(plan));
          lease.projectRevision = fixture.projectRevision + 1;
          lease.head = _head(fixture.projectRevision + 1);
          return Revision3QuestContextEditPublication(
            projectId: revision3QuestOutlineProjectId,
            projectRevision: fixture.projectRevision + 1,
            questId: revision3QuestOutlineQuestId,
            moduleId: revision3QuestOutlineModuleId,
            questRevision: fixture.questRevision + 1,
            moduleRevision: fixture.moduleRevision + 1,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final loaded = await coordinator.readCurrentRevision3QuestContextSeed(
        expectedRoot: managed.root.path,
        expectedProjectId: revision3QuestOutlineProjectId,
        expectedProjectRevision: fixture.projectRevision,
        expectedHead: _head(fixture.projectRevision),
        questId: revision3QuestOutlineQuestId,
        expectedQuestRevision: fixture.questRevision,
        expectedModuleId: revision3QuestOutlineModuleId,
        expectedModuleRevision: fixture.moduleRevision,
        expectedParentRuntimeClass: 'UQuest_SwampCamp_SCChapter2',
        expectedGiverRuntimeUniqueName: 'OM_GRD_Asghan_263',
      );
      expect(loaded.description, seed.description);
      expect(managed.questContextSeedCalls, 1);

      final publication = await coordinator.editCurrentRevision3QuestContext(
        expectedRoot: managed.root.path,
        expectedProjectId: revision3QuestOutlineProjectId,
        expectedProjectRevision: fixture.projectRevision,
        expectedHead: _head(fixture.projectRevision),
        gameRoot: r'C:\Games\Gothic Remake',
        plan: plan,
      );
      expect(publication.projectRevision, fixture.projectRevision + 1);
      expect(managed.questContextPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        fixture.projectRevision + 1,
      );
    },
  );

  test(
    'Quest context stale and reopen guards make no extra lease calls',
    () async {
      final fixture = Revision3QuestOutlineFixture();
      final plan = await _questContextPlan(fixture);
      final managed = _FakeManagedLease(
        root: Directory('managed-quest-context-guards'),
        projectIdValue: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision,
        head: _head(fixture.projectRevision),
        onQuestContextPublish: (lease, _, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected context verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.editCurrentRevision3QuestContext(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: fixture.projectRevision,
          expectedHead: _head(fixture.projectRevision - 1),
          gameRoot: r'C:\Games\Gothic Remake',
          plan: plan,
        ),
        throwsA(isA<Revision3QuestContextStaleCheckpointException>()),
      );
      expect(managed.questContextPublishCalls, 0);

      await expectLater(
        coordinator.editCurrentRevision3QuestContext(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: fixture.projectRevision,
          expectedHead: _head(fixture.projectRevision),
          gameRoot: r'C:\Games\Gothic Remake',
          plan: plan,
        ),
        throwsA(isA<Revision3QuestContextRequiresReopenException>()),
      );
      expect(managed.questContextPublishCalls, 1);
      await expectLater(
        coordinator.editCurrentRevision3QuestContext(
          expectedRoot: managed.root.path,
          expectedProjectId: revision3QuestOutlineProjectId,
          expectedProjectRevision: fixture.projectRevision,
          expectedHead: _head(fixture.projectRevision),
          gameRoot: r'C:\Games\Gothic Remake',
          plan: plan,
        ),
        throwsA(isA<Revision3QuestContextRequiresReopenException>()),
      );
      expect(managed.questContextPublishCalls, 1);
    },
  );

  test(
    'NPC publication is exact-checkpoint bound and refreshes R3 state',
    () async {
      const projectId = '18181818181818181818181818181818';
      late Revision3NpcDraftAuthoringInput receivedInput;
      final managed = _FakeManagedLease(
        root: Directory('managed-npc'),
        projectIdValue: projectId,
        projectRevision: 18,
        head: _head(18),
        onNpcPublish: (lease, gameRoot, input) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          receivedInput = input;
          lease.projectRevision = 19;
          lease.head = _head(19);
          return Revision3NpcDraftPublication(
            projectId: projectId,
            projectRevision: 19,
            npcId: '28282828282828282828282828282828',
            scriptModuleId: '38383838383838383838383838383838',
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final published = await coordinator.createCurrentRevision3NpcDraft(
        expectedRoot: managed.root.path,
        expectedHead: _head(18),
        expectedProjectId: projectId,
        expectedProjectRevision: 18,
        gameRoot: r'C:\Games\Gothic Remake',
        input: _npcInput(),
      );

      expect(published.projectRevision, 19);
      expect(receivedInput.displayName, 'North Gate Guard');
      expect(receivedInput.parentCatalogId, 'g1r:npc:om_grd_asghan_263');
      expect(managed.npcPublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 19);
      expect(state.head.canonicalJson, _head(19).canonicalJson);
    },
  );

  test(
    'NPC publication rejects divergent root or head before lease access',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-npc-stale'),
        projectIdValue: '19191919191919191919191919191919',
        projectRevision: 19,
        head: _head(19),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      Future<void> publish({
        required String root,
        required AuthoringWorkingHead head,
      }) async {
        await coordinator.createCurrentRevision3NpcDraft(
          expectedRoot: root,
          expectedHead: head,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 19,
          gameRoot: r'C:\Games\Gothic Remake',
          input: _npcInput(),
        );
      }

      await expectLater(
        publish(
          root: Directory('divergent-managed-clone').path,
          head: _head(19),
        ),
        throwsA(isA<Revision3NpcDraftStaleCheckpointException>()),
      );
      await expectLater(
        publish(root: managed.root.path, head: _head(1919)),
        throwsA(isA<Revision3NpcDraftStaleCheckpointException>()),
      );
      expect(managed.npcPublishCalls, 0);
    },
  );

  test(
    'NPC publication rejects an empty game root before lease access',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-npc-no-game'),
        projectIdValue: '20202020202020202020202020202020',
        projectRevision: 20,
        head: _head(20),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.createCurrentRevision3NpcDraft(
          expectedRoot: managed.root.path,
          expectedHead: _head(20),
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 20,
          gameRoot: '',
          input: _npcInput(),
        ),
        throwsA(isA<CurrentProjectOperationUnsupportedException>()),
      );
      expect(managed.npcPublishCalls, 0);
    },
  );

  test(
    'poisoned NPC failure locks retries and refreshes requiresReopen',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-poisoned-npc'),
        projectIdValue: '21212121212121212121212121212121',
        projectRevision: 21,
        head: _head(21),
        onNpcPublish: (lease, _, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected NPC publication verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      Future<void> publish() async {
        await coordinator.createCurrentRevision3NpcDraft(
          expectedRoot: managed.root.path,
          expectedHead: _head(21),
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 21,
          gameRoot: r'C:\Games\Gothic Remake',
          input: _npcInput(),
        );
      }

      await expectLater(
        publish(),
        throwsA(isA<Revision3NpcDraftRequiresReopenException>()),
      );
      expect(managed.npcPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        publish(),
        throwsA(isA<Revision3NpcDraftRequiresReopenException>()),
      );
      expect(managed.npcPublishCalls, 1);
    },
  );

  test(
    'Voice publication is exact-checkpoint bound and refreshes R3 state',
    () async {
      final plan = _voicePlan();
      final managed = _FakeManagedLease(
        root: Directory('managed-voice'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onVoicePublish: (lease, gameRoot, received) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          expect(received, same(plan));
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3VoiceTakePublication(
            projectId: revision3VoiceContentProjectId,
            projectRevision: 8,
            lineId: received.lineId,
            slotId: received.slotId,
            takeId: received.takeId,
            slotCreated: received.expectsSlotCreated,
            selected: received.selectTake,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.addCurrentRevision3VoiceTake(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 7,
          expectedHead: _head(6),
          gameRoot: r'C:\Games\Gothic Remake',
          plan: plan,
        ),
        throwsA(isA<Revision3VoiceTakeStaleCheckpointException>()),
      );
      expect(managed.voicePublishCalls, 0);

      await expectLater(
        coordinator.addCurrentRevision3VoiceTake(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 7,
          expectedHead: _head(7),
          gameRoot: '',
          plan: plan,
        ),
        throwsA(isA<CurrentProjectOperationUnsupportedException>()),
      );
      expect(managed.voicePublishCalls, 0);

      final publication = await coordinator.addCurrentRevision3VoiceTake(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        gameRoot: r'C:\Games\Gothic Remake',
        plan: plan,
      );
      expect(publication.takeId, plan.takeId);
      expect(managed.voicePublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
    },
  );

  test('poisoned Voice failure locks retries behind requires-reopen', () async {
    final plan = _voicePlan();
    final managed = _FakeManagedLease(
      root: Directory('managed-poisoned-voice'),
      projectIdValue: revision3VoiceContentProjectId,
      projectRevision: 7,
      head: _head(7),
      onVoicePublish: (lease, _, _) {
        lease.requiresReopenValue = true;
        throw StateError('injected Voice publication verification failure');
      },
    );
    final coordinator = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => managed,
    );
    addTearDown(() async {
      await coordinator.shutdown();
      coordinator.dispose();
    });
    await coordinator.openManagedRevision3(managed.root);

    Future<void> publish() async {
      await coordinator.addCurrentRevision3VoiceTake(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        gameRoot: r'C:\Games\Gothic Remake',
        plan: plan,
      );
    }

    await expectLater(
      publish(),
      throwsA(isA<Revision3VoiceTakeRequiresReopenException>()),
    );
    expect(managed.voicePublishCalls, 1);
    expect(
      (coordinator.state as ManagedRevision3CurrentProjectState).requiresReopen,
      isTrue,
    );
    await expectLater(
      publish(),
      throwsA(isA<Revision3VoiceTakeRequiresReopenException>()),
    );
    expect(managed.voicePublishCalls, 1);
  });

  test(
    'Voice selection is exact-checkpoint bound, needs no game root, and refreshes state',
    () async {
      final plan = _voiceSelectionPlan();
      final managed = _FakeManagedLease(
        root: Directory('managed-voice-selection'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onVoiceSelectionPublish: (lease, received) {
          expect(received, same(plan));
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3VoiceTakeSelectionPublication(
            projectId: revision3VoiceContentProjectId,
            projectRevision: 8,
            lineId: received.lineId,
            slotId: received.slotId,
            slotRevision: received.expectedSlotRevision + 1,
            locale: received.locale,
            locId: received.locId,
            previousSelectedTakeId: received.expectedSelectedTakeId,
            selectedTakeId: received.selectedTakeId,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      for (final stale
          in <({String root, int revision, AuthoringWorkingHead head})>[
            (root: Directory('another-root').path, revision: 7, head: _head(7)),
            (root: managed.root.path, revision: 6, head: _head(7)),
            (root: managed.root.path, revision: 7, head: _head(6)),
          ]) {
        await expectLater(
          coordinator.selectCurrentRevision3VoiceTake(
            expectedRoot: stale.root,
            expectedProjectId: managed.projectId,
            expectedProjectRevision: stale.revision,
            expectedHead: stale.head,
            plan: plan,
          ),
          throwsA(isA<Revision3VoiceTakeSelectionStaleCheckpointException>()),
        );
      }
      expect(managed.voiceSelectionPublishCalls, 0);

      final publication = await coordinator.selectCurrentRevision3VoiceTake(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        plan: plan,
      );
      expect(publication.selectedTakeId, isNull);
      expect(managed.voiceSelectionPublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 8);
      expect(state.head.canonicalJson, _head(8).canonicalJson);
    },
  );

  test(
    'poisoned Voice selection maps to requires-reopen and locks retry',
    () async {
      final plan = _voiceSelectionPlan();
      final managed = _FakeManagedLease(
        root: Directory('managed-poisoned-voice-selection'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onVoiceSelectionPublish: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected Voice selection verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      Future<void> publish() async {
        await coordinator.selectCurrentRevision3VoiceTake(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 7,
          expectedHead: _head(7),
          plan: plan,
        );
      }

      await expectLater(
        publish(),
        throwsA(isA<Revision3VoiceTakeSelectionRequiresReopenException>()),
      );
      expect(managed.voiceSelectionPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        publish(),
        throwsA(isA<Revision3VoiceTakeSelectionRequiresReopenException>()),
      );
      expect(managed.voiceSelectionPublishCalls, 1);
    },
  );

  test(
    'Voice target publication advances exactly once and Voice build preserves the checkpoint',
    () async {
      final plan = _voiceTargetPlan();
      final managed = _FakeManagedLease(
        root: Directory('managed-voice-target-build'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onVoiceTargetPublish: (lease, gameRoot, received) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          expect(received, same(plan));
          lease.projectRevision = 8;
          lease.head = _head(8);
          return Revision3VoiceTargetPublication(
            projectId: revision3VoiceContentProjectId,
            projectRevision: 8,
            lineId: received.lineId,
            slotId: received.slotId,
            locale: received.locale,
            locId: received.locId,
            resolution: AuthoringRevision3VoiceTargetResolutionState.resolved,
            matchCount: 1,
          );
        },
        onVoiceBuild: (lease, gameRoot, output) {
          expect(gameRoot, r'C:\Games\Gothic Remake');
          return _voiceBuildResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            output: output,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.resolveCurrentRevision3VoiceTarget(
          expectedRoot: managed.root.path,
          expectedProjectId: managed.projectId,
          expectedProjectRevision: 6,
          expectedHead: _head(6),
          gameRoot: r'C:\Games\Gothic Remake',
          plan: plan,
        ),
        throwsA(isA<Revision3VoiceTargetStaleCheckpointException>()),
      );
      expect(managed.voiceTargetPublishCalls, 0);

      final publication = await coordinator.resolveCurrentRevision3VoiceTarget(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        gameRoot: r'C:\Games\Gothic Remake',
        plan: plan,
      );
      expect(
        publication.resolution,
        AuthoringRevision3VoiceTargetResolutionState.resolved,
      );
      expect(managed.voiceTargetPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        8,
      );

      const output = r'C:\Builds\managed-voice';
      final build = await coordinator.buildCurrentRevision3Voice(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 8,
        expectedHead: _head(8),
        gameRoot: r'C:\Games\Gothic Remake',
        output: output,
      );
      expect(build.isBuilt, isTrue);
      expect(build.output, output);
      expect(managed.voiceBuildCalls, 1);
      expect(managed.projectRevision, 8);
      expect(managed.head.canonicalJson, _head(8).canonicalJson);
    },
  );

  test(
    'DataAsset list is exact-checkpoint bound before lease access',
    () async {
      final stage = _dataAssetStage();
      final managed = _FakeManagedLease(
        root: Directory('managed-dataasset-list'),
        projectIdValue: stage.projectId,
        projectRevision: 5,
        head: _head(5),
        dataAssetStages: [stage],
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.listCurrentRevision3DataAssetStages(
          expectedRoot: managed.root.path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 4,
          expectedHead: _head(4),
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );
      expect(managed.dataAssetListCalls, 0);

      await expectLater(
        coordinator.listCurrentRevision3DataAssetStages(
          expectedRoot: Directory('divergent-managed-clone').path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 5,
          expectedHead: _head(5),
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );
      await expectLater(
        coordinator.listCurrentRevision3DataAssetStages(
          expectedRoot: managed.root.path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 5,
          expectedHead: _head(55),
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );
      expect(managed.dataAssetListCalls, 0);

      final listed = await coordinator.listCurrentRevision3DataAssetStages(
        expectedRoot: managed.root.path,
        expectedProjectId: stage.projectId,
        expectedProjectRevision: 5,
        expectedHead: _head(5),
      );
      expect(listed, [same(stage)]);
      expect(managed.dataAssetListCalls, 1);
    },
  );

  test(
    'Voice build returns a basis receipt while publishing reopen state',
    () async {
      final managed = _FakeManagedLease(
        root: Directory('managed-voice-snapshot-build'),
        projectIdValue: revision3VoiceContentProjectId,
        projectRevision: 7,
        head: _head(7),
        onVoiceBuild: (lease, _, output) {
          final result = _voiceBuildResult(
            head: lease.head,
            projectId: lease.projectId,
            projectRevision: lease.projectRevision,
            output: output,
          );
          lease.requiresReopenValue = true;
          return result;
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      const output = r'C:\Builds\snapshot-voice';
      final result = await coordinator.buildCurrentRevision3Voice(
        expectedRoot: managed.root.path,
        expectedProjectId: managed.projectId,
        expectedProjectRevision: 7,
        expectedHead: _head(7),
        gameRoot: r'C:\Games\Gothic Remake',
        output: output,
      );

      expect(result.output, output);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
    },
  );

  test(
    'DataAsset add and registry removal advance and refresh R3 state',
    () async {
      final stage = _dataAssetStage();
      late _FakeManagedLease managed;
      managed = _FakeManagedLease(
        root: Directory('managed-dataasset-write'),
        projectIdValue: stage.projectId,
        projectRevision: 4,
        head: _head(4),
        onDataAssetPublish: (lease, receiptPath) {
          expect(receiptPath, r'C:\proof\edit.gore-asset-patch.json');
          lease.projectRevision = 5;
          lease.head = _head(5);
          return Revision3DataAssetStagePublication(
            projectId: stage.projectId,
            projectRevision: 5,
            stage: stage,
            deduplicatedBlobs: 1,
          );
        },
        onDataAssetRemove: (lease, targetPath) {
          expect(targetPath, stage.targetPath);
          lease.projectRevision = 6;
          lease.head = _head(6);
          return Revision3DataAssetStageRemovalPublication(
            projectId: stage.projectId,
            projectRevision: 6,
            removed: stage,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final added = await coordinator.addCurrentRevision3DataAssetStage(
        expectedRoot: managed.root.path,
        expectedProjectId: stage.projectId,
        expectedProjectRevision: 4,
        expectedHead: _head(4),
        patchReceiptPath: r'C:\proof\edit.gore-asset-patch.json',
      );
      expect(added.stage, same(stage));
      expect(managed.dataAssetPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        5,
      );

      final removed = await coordinator.removeCurrentRevision3DataAssetStage(
        expectedRoot: managed.root.path,
        expectedProjectId: stage.projectId,
        expectedProjectRevision: 5,
        expectedHead: _head(5),
        targetPath: stage.targetPath,
      );
      expect(removed.removed, same(stage));
      expect(managed.dataAssetRemoveCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .projectRevision,
        6,
      );
    },
  );

  test(
    'typed DataAsset value edit is exact-checkpoint bound and refreshes state',
    () async {
      final stage = _dataAssetStage();
      final intent = _dataAssetSemanticIntent();
      late _FakeManagedLease managed;
      managed = _FakeManagedLease(
        root: Directory('managed-dataasset-semantic'),
        projectIdValue: stage.projectId,
        projectRevision: 4,
        head: _head(4),
        onDataAssetSemanticPublish: (lease, received) {
          expect(received, same(intent));
          expect(
            received.toNativeFields()['extract_receipt_path'],
            r'C:\proof\extract-receipt.v2.json',
          );
          lease.projectRevision = 5;
          lease.head = _head(5);
          return Revision3DataAssetStagePublication(
            projectId: stage.projectId,
            projectRevision: 5,
            stage: stage,
            deduplicatedBlobs: 0,
          );
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      final published = await coordinator.addCurrentRevision3DataAssetEdit(
        expectedRoot: managed.root.path,
        expectedProjectId: stage.projectId,
        expectedProjectRevision: 4,
        expectedHead: _head(4),
        intent: intent,
      );

      expect(published.stage, same(stage));
      expect(managed.dataAssetSemanticPublishCalls, 1);
      final state = coordinator.state as ManagedRevision3CurrentProjectState;
      expect(state.projectRevision, 5);
      expect(state.head.canonicalJson, _head(5).canonicalJson);

      await expectLater(
        coordinator.addCurrentRevision3DataAssetEdit(
          expectedRoot: managed.root.path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 4,
          expectedHead: _head(4),
          intent: intent,
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );
      expect(managed.dataAssetSemanticPublishCalls, 1);
    },
  );

  test(
    'DataAsset mutations reject divergent roots and heads before lease access',
    () async {
      final stage = _dataAssetStage();
      final managed = _FakeManagedLease(
        root: Directory('managed-dataasset-exact-mutation'),
        projectIdValue: stage.projectId,
        projectRevision: 5,
        head: _head(5),
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      await expectLater(
        coordinator.addCurrentRevision3DataAssetStage(
          expectedRoot: managed.root.path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 5,
          expectedHead: _head(55),
          patchReceiptPath: r'C:\proof\edit.gore-asset-patch.json',
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );
      await expectLater(
        coordinator.removeCurrentRevision3DataAssetStage(
          expectedRoot: Directory('divergent-managed-clone').path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 5,
          expectedHead: _head(5),
          targetPath: stage.targetPath,
        ),
        throwsA(isA<Revision3DataAssetStaleCheckpointException>()),
      );

      expect(managed.dataAssetPublishCalls, 0);
      expect(managed.dataAssetRemoveCalls, 0);
    },
  );

  test(
    'poisoned DataAsset failure locks retries and refreshes state',
    () async {
      final stage = _dataAssetStage();
      final managed = _FakeManagedLease(
        root: Directory('managed-dataasset-poisoned'),
        projectIdValue: stage.projectId,
        projectRevision: 4,
        head: _head(4),
        onDataAssetPublish: (lease, _) {
          lease.requiresReopenValue = true;
          throw StateError('injected DataAsset verification failure');
        },
      );
      final coordinator = CurrentProjectCoordinator(
        openManagedRevision3: (_) async => managed,
      );
      addTearDown(() async {
        await coordinator.shutdown();
        coordinator.dispose();
      });
      await coordinator.openManagedRevision3(managed.root);

      Future<void> add() async {
        await coordinator.addCurrentRevision3DataAssetStage(
          expectedRoot: managed.root.path,
          expectedProjectId: stage.projectId,
          expectedProjectRevision: 4,
          expectedHead: _head(4),
          patchReceiptPath: r'C:\proof\edit.gore-asset-patch.json',
        );
      }

      await expectLater(
        add(),
        throwsA(isA<Revision3DataAssetRequiresReopenException>()),
      );
      expect(managed.dataAssetPublishCalls, 1);
      expect(
        (coordinator.state as ManagedRevision3CurrentProjectState)
            .requiresReopen,
        isTrue,
      );
      await expectLater(
        add(),
        throwsA(isA<Revision3DataAssetRequiresReopenException>()),
      );
      expect(managed.dataAssetPublishCalls, 1);
    },
  );

  test('content read rejects absent and legacy current projects', () async {
    final empty = CurrentProjectCoordinator(
      openManagedRevision3: (_) async => throw UnimplementedError(),
    );
    addTearDown(() async {
      await empty.shutdown();
      empty.dispose();
    });
    await expectLater(
      empty.readCurrentRevision3ContentIndex(),
      throwsA(isA<NoCurrentProjectException>()),
    );

    final legacy = CurrentProjectCoordinator(
      initialLegacy: _FakeLegacyLease(path: 'legacy.goremod'),
      openManagedRevision3: (_) async => throw UnimplementedError(),
    );
    addTearDown(() async {
      await legacy.shutdown();
      legacy.dispose();
    });
    await expectLater(
      legacy.readCurrentRevision3ContentIndex(),
      throwsA(isA<CurrentProjectOperationUnsupportedException>()),
    );
  });
}

final class _FakeLegacyLease implements LegacyCurrentProjectLease {
  _FakeLegacyLease({
    this.path,
    this.hasUnsavedChanges = false,
    this.onSave,
    this.onOpenFromPath,
    this.onClose,
  });

  String? path;
  @override
  final bool hasUnsavedChanges;
  final FutureOr<void> Function()? onSave;
  final FutureOr<void> Function(String path)? onOpenFromPath;
  final FutureOr<void> Function()? onClose;
  int saveCalls = 0;
  int saveToPathCalls = 0;
  int openFromPathCalls = 0;
  int newProjectCalls = 0;
  int closeCalls = 0;

  @override
  String? get currentPath => path;

  @override
  Future<void> saveCurrent() async {
    saveCalls++;
    await onSave?.call();
  }

  @override
  Future<void> saveToPath(String path) async {
    saveToPathCalls++;
    this.path = path;
  }

  @override
  Future<void> openFromPath(String path) async {
    openFromPathCalls++;
    await onOpenFromPath?.call(path);
    this.path = path;
  }

  @override
  Future<void> newProject() async {
    newProjectCalls++;
    path = null;
  }

  @override
  Future<void> close() async {
    closeCalls++;
    await onClose?.call();
  }
}

AuthoringRevision3QuestTransitionsSeed _questTransitionsSeed(
  Revision3QuestOutlineFixture fixture,
) => AuthoringRevision3QuestTransitionsSeed.forProject(
  currentProjectJson: fixture.projectJson,
  questId: revision3QuestOutlineQuestId,
  expectedQuestRevision: fixture.questRevision,
  expectedModuleId: revision3QuestOutlineModuleId,
  expectedModuleRevision: fixture.moduleRevision,
);

Future<Revision3QuestTransitionsEditTechnicalPlan> _questTransitionsPlan(
  Revision3QuestOutlineFixture fixture,
) async {
  final index = fixture.contentIndex();
  final seed = _questTransitionsSeed(fixture);
  Revision3QuestTransitionsEditTechnicalPlan? result;
  final service = Revision3QuestTransitionsAuthoringService(
    loadSeed:
        ({
          required questId,
          required expectedQuestRevision,
          required expectedModuleId,
          required expectedModuleRevision,
        }) async => seed,
    publishTechnicalPlan: ({required plan}) async {
      result = plan;
      return Revision3QuestTransitionsEditPublication(
        projectId: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision + 1,
        questId: revision3QuestOutlineQuestId,
        moduleId: revision3QuestOutlineModuleId,
        questRevision: fixture.questRevision + 1,
        moduleRevision: fixture.moduleRevision + 1,
        transitionPlanSeal: plan.transitionPlan.contentSeal,
      );
    },
  );
  final checkpoint = await service.load(
    index: index,
    quest: index.entityById(revision3QuestOutlineQuestId)!,
  );
  await service.publish(
    checkpoint: checkpoint,
    transitionPlan:
        Revision3QuestTransitionsAuthoringService.sequentialTemplate(
          seed.transitionPlan,
        ),
  );
  return result!;
}

AuthoringRevision3QuestContextSeed _questContextSeed(
  Revision3QuestOutlineFixture fixture,
) => AuthoringRevision3QuestContextSeed.forProject(
  currentProjectJson: fixture.projectJson,
  questId: revision3QuestOutlineQuestId,
  expectedQuestRevision: fixture.questRevision,
  expectedModuleId: revision3QuestOutlineModuleId,
  expectedModuleRevision: fixture.moduleRevision,
  expectedParentRuntimeClass: 'UQuest_SwampCamp_SCChapter2',
  expectedGiverRuntimeUniqueName: 'OM_GRD_Asghan_263',
);

Future<Revision3QuestContextEditTechnicalPlan> _questContextPlan(
  Revision3QuestOutlineFixture fixture,
) async {
  final index = fixture.contentIndex();
  final catalog = _controllerQuestContextCatalog(fixture);
  Revision3QuestContextEditTechnicalPlan? result;
  final service = Revision3QuestContextAuthoringService(
    loadSeed:
        ({
          required questId,
          required expectedQuestRevision,
          required expectedModuleId,
          required expectedModuleRevision,
          required expectedParentRuntimeClass,
          required expectedGiverRuntimeUniqueName,
        }) async => _questContextSeed(fixture),
    loadCatalog: (_) async => catalog,
    publishTechnicalPlan: ({required gameRoot, required plan}) async {
      result = plan;
      return Revision3QuestContextEditPublication(
        projectId: revision3QuestOutlineProjectId,
        projectRevision: fixture.projectRevision + 1,
        questId: revision3QuestOutlineQuestId,
        moduleId: revision3QuestOutlineModuleId,
        questRevision: fixture.questRevision + 1,
        moduleRevision: fixture.moduleRevision + 1,
      );
    },
  );
  final checkpoint = await service.load(
    index: index,
    quest: index.entityById(revision3QuestOutlineQuestId)!,
    gameRoot: r'C:\Games\Gothic Remake',
  );
  await service.publish(
    checkpoint: checkpoint,
    gameRoot: r'C:\Games\Gothic Remake',
    description: 'Find Homer and report back safely.',
    parent: checkpoint.catalog.parent(revision3QuestContextParentCatalogId)!,
    giver: checkpoint.catalog.giver(revision3QuestContextGiverCatalogId)!,
  );
  return result!;
}

Revision3QuestCatalog _controllerQuestContextCatalog(
  Revision3QuestOutlineFixture fixture,
) => Revision3QuestCatalog(
  parents: [
    Revision3QuestParentChoice(
      catalogId: 'current-parent',
      displayName: 'Chapter Two',
      runtimeClass: 'UQuest_SwampCamp_SCChapter2',
      catalogLayer: 'base-game.quest-parent.v1',
      authoringSelector: 'SwampCamp_SCChapter2',
      sourceSeal: _controllerSeal(11, '1'),
    ),
    Revision3QuestParentChoice(
      catalogId: revision3QuestContextParentCatalogId,
      displayName: 'Chapter Three',
      runtimeClass: revision3QuestContextParentRuntimeClass,
      catalogLayer: 'base-game.quest-parent.v1',
      authoringSelector: 'SwampCamp_SCChapter3',
      sourceSeal: _controllerSeal(11, '1'),
    ),
  ],
  givers: [
    Revision3QuestGiverChoice(
      catalogId: 'current-giver',
      displayName: 'Asghan',
      runtimeUniqueName: 'OM_GRD_Asghan_263',
      catalogLayer: 'base-game.npc.v1',
      authoringSelector: 'OM_GRD_Asghan_263',
      sourceSeal: _controllerSeal(12, '2'),
    ),
    Revision3QuestGiverChoice(
      catalogId: revision3QuestContextGiverCatalogId,
      displayName: 'Viper',
      runtimeUniqueName: revision3QuestContextGiverRuntimeUniqueName,
      catalogLayer: 'base-game.npc.v1',
      authoringSelector: revision3QuestContextGiverRuntimeUniqueName,
      sourceSeal: _controllerSeal(12, '2'),
    ),
  ],
  catalogSeal: fixture.storyCatalogSeal,
  generationExecutableSeal: _controllerSeal(171698176, 'a'),
);

AuthoringDraftContentSeal _controllerSeal(int bytes, String digit) =>
    AuthoringDraftContentSeal.fromJson(<String, Object?>{
      'byte_len': bytes,
      'sha256': List<String>.filled(64, digit).join(),
    });

typedef _VerifyHook = FutureOr<void> Function(_FakeManagedLease lease);
typedef _ContentReadHook = FutureOr<void> Function(_FakeManagedLease lease);
typedef _QuestInspectionHook =
    FutureOr<AuthoringRevision3QuestSourceInspectionResult> Function(
      _FakeManagedLease lease,
      String gameRoot,
      String questId,
    );
typedef _NpcInspectionHook =
    FutureOr<AuthoringRevision3NpcSourceInspectionResult> Function(
      _FakeManagedLease lease,
      String npcId,
    );
typedef _QuestPublishHook =
    FutureOr<Revision3QuestDraftPublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      Revision3QuestDraftAuthoringInput input,
    );
typedef _QuestOutlinePublishHook =
    FutureOr<Revision3QuestOutlineEditPublication> Function(
      _FakeManagedLease lease,
      Revision3QuestOutlineEditInput input,
    );
typedef _QuestTransitionsSeedHook =
    FutureOr<AuthoringRevision3QuestTransitionsSeed> Function(
      _FakeManagedLease lease,
      String questId,
      int questRevision,
      String moduleId,
      int moduleRevision,
    );
typedef _QuestTransitionsPublishHook =
    FutureOr<Revision3QuestTransitionsEditPublication> Function(
      _FakeManagedLease lease,
      Revision3QuestTransitionsEditTechnicalPlan plan,
    );
typedef _QuestContextSeedHook =
    FutureOr<AuthoringRevision3QuestContextSeed> Function(
      _FakeManagedLease lease,
      String questId,
      int questRevision,
      String moduleId,
      int moduleRevision,
      String parentRuntimeClass,
      String giverRuntimeUniqueName,
    );
typedef _QuestContextPublishHook =
    FutureOr<Revision3QuestContextEditPublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      Revision3QuestContextEditTechnicalPlan plan,
    );
typedef _NpcPublishHook =
    FutureOr<Revision3NpcDraftPublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      Revision3NpcDraftAuthoringInput input,
    );
typedef _VoicePublishHook =
    FutureOr<Revision3VoiceTakePublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      Revision3VoiceTakeTechnicalPlan plan,
    );
typedef _VoiceTargetPublishHook =
    FutureOr<Revision3VoiceTargetPublication> Function(
      _FakeManagedLease lease,
      String gameRoot,
      Revision3VoiceTargetTechnicalPlan plan,
    );
typedef _VoiceBuildHook =
    FutureOr<AuthoringRevision3VoiceBuildResult> Function(
      _FakeManagedLease lease,
      String gameRoot,
      String output,
    );
typedef _DataAssetPublishHook =
    FutureOr<Revision3DataAssetStagePublication> Function(
      _FakeManagedLease lease,
      String patchReceiptPath,
    );
typedef _DataAssetSemanticPublishHook =
    FutureOr<Revision3DataAssetStagePublication> Function(
      _FakeManagedLease lease,
      DataAssetSemanticEditIntent intent,
    );
typedef _VoiceSelectionPublishHook =
    FutureOr<Revision3VoiceTakeSelectionPublication> Function(
      _FakeManagedLease lease,
      Revision3VoiceTakeSelectionTechnicalPlan plan,
    );
typedef _DataAssetRemoveHook =
    FutureOr<Revision3DataAssetStageRemovalPublication> Function(
      _FakeManagedLease lease,
      String targetPath,
    );
typedef _DataAssetPackageIndexReadHook =
    FutureOr<AuthoringRevision3DataAssetPackageIndexResult> Function(
      _FakeManagedLease lease,
      String gameRoot,
    );

final class _FakeManagedLease implements ManagedRevision3CurrentProjectLease {
  _FakeManagedLease({
    required this.root,
    required this.projectIdValue,
    required this.projectRevision,
    required this.head,
    this.projectIdError,
    this.onVerify,
    this.onQuestInspection,
    this.onNpcInspection,
    this.onNpcPublish,
    this.onQuestPublish,
    this.onQuestOutlinePublish,
    this.onQuestTransitionsSeed,
    this.onQuestTransitionsPublish,
    this.onQuestContextSeed,
    this.onQuestContextPublish,
    this.onVoicePublish,
    this.onVoiceSelectionPublish,
    this.onVoiceTargetPublish,
    this.onVoiceBuild,
    this.onDataAssetPublish,
    this.onDataAssetSemanticPublish,
    this.onDataAssetRemove,
    this.onDataAssetPackageIndexRead,
    this.dataAssetStages = const [],
    this.contentIndex,
    this.closeFailuresRemaining = 0,
  });

  @override
  final Directory root;
  final String projectIdValue;
  final Object? projectIdError;
  @override
  int projectRevision;
  @override
  AuthoringWorkingHead head;
  final _VerifyHook? onVerify;
  final _QuestInspectionHook? onQuestInspection;
  final _NpcInspectionHook? onNpcInspection;
  final _NpcPublishHook? onNpcPublish;
  final _QuestPublishHook? onQuestPublish;
  final _QuestOutlinePublishHook? onQuestOutlinePublish;
  final _QuestTransitionsSeedHook? onQuestTransitionsSeed;
  final _QuestTransitionsPublishHook? onQuestTransitionsPublish;
  final _QuestContextSeedHook? onQuestContextSeed;
  final _QuestContextPublishHook? onQuestContextPublish;
  final _VoicePublishHook? onVoicePublish;
  final _VoiceSelectionPublishHook? onVoiceSelectionPublish;
  final _VoiceTargetPublishHook? onVoiceTargetPublish;
  final _VoiceBuildHook? onVoiceBuild;
  final _DataAssetPublishHook? onDataAssetPublish;
  final _DataAssetSemanticPublishHook? onDataAssetSemanticPublish;
  final _DataAssetRemoveHook? onDataAssetRemove;
  final _DataAssetPackageIndexReadHook? onDataAssetPackageIndexRead;
  final List<AuthoringRevision3DataAssetStage> dataAssetStages;
  final Revision3ContentIndex? contentIndex;
  _ContentReadHook? onContentRead;
  int closeFailuresRemaining;
  bool requiresReopenValue = false;
  int verifyCalls = 0;
  int contentReadCalls = 0;
  int questInspectionCalls = 0;
  final List<String> questInspectionGameRoots = <String>[];
  final List<String> questInspectionQuestIds = <String>[];
  int npcInspectionCalls = 0;
  final List<String> npcInspectionNpcIds = <String>[];
  int npcPublishCalls = 0;
  int questPublishCalls = 0;
  int questOutlinePublishCalls = 0;
  int questTransitionsSeedCalls = 0;
  int questTransitionsPublishCalls = 0;
  int questContextSeedCalls = 0;
  int questContextPublishCalls = 0;
  int voicePublishCalls = 0;
  int voiceSelectionPublishCalls = 0;
  int voiceTargetPublishCalls = 0;
  int voiceBuildCalls = 0;
  int dataAssetListCalls = 0;
  int dataAssetPublishCalls = 0;
  int dataAssetSemanticPublishCalls = 0;
  int dataAssetRemoveCalls = 0;
  int dataAssetPackageIndexReadCalls = 0;
  final List<String> dataAssetPackageIndexGameRoots = <String>[];
  int closeCalls = 0;

  @override
  String get projectId {
    final error = projectIdError;
    if (error != null) throw error;
    return projectIdValue;
  }

  @override
  bool get requiresReopen => requiresReopenValue;

  @override
  Future<void> verifyCurrentHead() async {
    verifyCalls++;
    await onVerify?.call(this);
  }

  @override
  Future<Revision3ContentIndex> readContentIndex() async {
    contentReadCalls++;
    await onContentRead?.call(this);
    return contentIndex ??
        (throw StateError('fake managed lease has no content index'));
  }

  @override
  Future<AuthoringRevision3QuestSourceInspectionResult> inspectQuestSourceV1({
    required String gameRoot,
    required String questId,
  }) async {
    questInspectionCalls++;
    questInspectionGameRoots.add(gameRoot);
    questInspectionQuestIds.add(questId);
    final inspect = onQuestInspection;
    if (inspect == null) {
      throw StateError('fake managed lease has no Quest source inspector');
    }
    return inspect(this, gameRoot, questId);
  }

  @override
  Future<AuthoringRevision3NpcSourceInspectionResult> inspectNpcSourceV1({
    required String npcId,
  }) async {
    npcInspectionCalls++;
    npcInspectionNpcIds.add(npcId);
    final inspect = onNpcInspection;
    if (inspect == null) {
      throw StateError('fake managed lease has no NPC source inspector');
    }
    return inspect(this, npcId);
  }

  @override
  Future<AuthoringRevision3DataAssetPackageIndexResult>
  readDataAssetPackageIndexV1({required String gameRoot}) async {
    dataAssetPackageIndexReadCalls++;
    dataAssetPackageIndexGameRoots.add(gameRoot);
    final read = onDataAssetPackageIndexRead;
    if (read == null) {
      throw StateError('fake managed lease has no DataAsset package index');
    }
    return read(this, gameRoot);
  }

  @override
  Future<Revision3QuestDraftPublication> prepareAndPublishQuestDraftV3({
    required String gameRoot,
    required Revision3QuestDraftAuthoringInput input,
  }) async {
    questPublishCalls++;
    final publish = onQuestPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Quest publisher');
    }
    return publish(this, gameRoot, input);
  }

  @override
  Future<Revision3QuestOutlineEditPublication>
  prepareAndPublishQuestOutlineEditV1({
    required Revision3QuestOutlineEditInput input,
  }) async {
    questOutlinePublishCalls++;
    final publish = onQuestOutlinePublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Quest outline publisher');
    }
    return publish(this, input);
  }

  @override
  Future<AuthoringRevision3QuestTransitionsSeed> readQuestTransitionsSeedV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
  }) async {
    questTransitionsSeedCalls++;
    final read = onQuestTransitionsSeed;
    if (read == null) {
      throw StateError('fake managed lease has no Quest transitions seed');
    }
    return read(
      this,
      questId,
      expectedQuestRevision,
      expectedModuleId,
      expectedModuleRevision,
    );
  }

  @override
  Future<Revision3QuestTransitionsEditPublication>
  prepareAndPublishQuestTransitionsEditV1({
    required Revision3QuestTransitionsEditTechnicalPlan plan,
  }) async {
    questTransitionsPublishCalls++;
    final publish = onQuestTransitionsPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Quest transitions publisher');
    }
    return publish(this, plan);
  }

  @override
  Future<AuthoringRevision3QuestContextSeed> readQuestContextSeedV1({
    required String questId,
    required int expectedQuestRevision,
    required String expectedModuleId,
    required int expectedModuleRevision,
    required String expectedParentRuntimeClass,
    required String expectedGiverRuntimeUniqueName,
  }) async {
    questContextSeedCalls++;
    final read = onQuestContextSeed;
    if (read == null) {
      throw StateError('fake managed lease has no Quest context seed');
    }
    return read(
      this,
      questId,
      expectedQuestRevision,
      expectedModuleId,
      expectedModuleRevision,
      expectedParentRuntimeClass,
      expectedGiverRuntimeUniqueName,
    );
  }

  @override
  Future<Revision3QuestContextEditPublication>
  prepareAndPublishQuestContextEditV1({
    required String gameRoot,
    required Revision3QuestContextEditTechnicalPlan plan,
  }) async {
    questContextPublishCalls++;
    final publish = onQuestContextPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Quest context publisher');
    }
    return publish(this, gameRoot, plan);
  }

  @override
  Future<Revision3NpcDraftPublication> prepareAndPublishNpcDraftV1({
    required String gameRoot,
    required Revision3NpcDraftAuthoringInput input,
  }) async {
    npcPublishCalls++;
    final publish = onNpcPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no NPC publisher');
    }
    return publish(this, gameRoot, input);
  }

  @override
  Future<Revision3VoiceTakePublication> prepareAndPublishVoiceTakeV1({
    required String gameRoot,
    required Revision3VoiceTakeTechnicalPlan plan,
  }) async {
    voicePublishCalls++;
    final publish = onVoicePublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Voice publisher');
    }
    return publish(this, gameRoot, plan);
  }

  @override
  Future<Revision3VoiceTakeSelectionPublication>
  prepareAndPublishVoiceTakeSelectionV1({
    required Revision3VoiceTakeSelectionTechnicalPlan plan,
  }) async {
    voiceSelectionPublishCalls++;
    final publish = onVoiceSelectionPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Voice selection publisher');
    }
    return publish(this, plan);
  }

  @override
  Future<Revision3VoiceTargetPublication> prepareAndPublishVoiceTargetV1({
    required String gameRoot,
    required Revision3VoiceTargetTechnicalPlan plan,
  }) async {
    voiceTargetPublishCalls++;
    final publish = onVoiceTargetPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no Voice target publisher');
    }
    return publish(this, gameRoot, plan);
  }

  @override
  Future<AuthoringRevision3VoiceBuildResult> buildVoiceV1({
    required String gameRoot,
    required String output,
  }) async {
    voiceBuildCalls++;
    final build = onVoiceBuild;
    if (build == null) {
      throw StateError('fake managed lease has no Voice builder');
    }
    return build(this, gameRoot, output);
  }

  @override
  Future<List<AuthoringRevision3DataAssetStage>> listDataAssetStagesV1() async {
    dataAssetListCalls++;
    return dataAssetStages;
  }

  @override
  Future<Revision3DataAssetStagePublication> prepareAndPublishDataAssetStageV1({
    required String patchReceiptPath,
  }) async {
    dataAssetPublishCalls++;
    final publish = onDataAssetPublish;
    if (publish == null) {
      throw StateError('fake managed lease has no DataAsset publisher');
    }
    return publish(this, patchReceiptPath);
  }

  @override
  Future<Revision3DataAssetStagePublication> prepareAndPublishDataAssetEditV1({
    required DataAssetSemanticEditIntent intent,
  }) async {
    dataAssetSemanticPublishCalls++;
    final publish = onDataAssetSemanticPublish;
    if (publish == null) {
      throw StateError(
        'fake managed lease has no semantic DataAsset publisher',
      );
    }
    return publish(this, intent);
  }

  @override
  Future<Revision3DataAssetStageRemovalPublication>
  prepareAndPublishRemoveDataAssetStageV1({required String targetPath}) async {
    dataAssetRemoveCalls++;
    final remove = onDataAssetRemove;
    if (remove == null) {
      throw StateError('fake managed lease has no DataAsset remover');
    }
    return remove(this, targetPath);
  }

  @override
  Future<void> close() async {
    closeCalls++;
    if (closeFailuresRemaining > 0) {
      closeFailuresRemaining--;
      throw StateError('injected close failure');
    }
  }
}

Revision3VoiceTakeTechnicalPlan _voicePlan() {
  final catalog = Revision3VoiceCatalog.fromContentIndex(
    revision3VoiceContentIndexFixture(),
  );
  return Revision3VoiceTakeTechnicalPlan.forCheckpoint(
    catalog: catalog,
    input: Revision3VoiceTakeAuthoringInput(
      lineId: revision3VoiceContentLineId,
      locale: 'de',
      sourcePath: r'C:\Voice\asghan.ogg',
      takeDisplayName: 'Asghan take',
      status: AuthoringRevision3VoiceTakeStatus.recorded,
    ),
  );
}

Revision3VoiceTargetTechnicalPlan _voiceTargetPlan() {
  final catalog = Revision3VoiceCatalog.fromContentIndex(
    revision3VoiceContentIndexFixture(),
  );
  return Revision3VoiceTargetTechnicalPlan.forCheckpoint(
    catalog: catalog,
    lineId: revision3VoiceContentLineId,
    locale: 'de',
  );
}

Revision3VoiceTakeSelectionTechnicalPlan _voiceSelectionPlan() {
  final catalog = Revision3VoiceCatalog.fromContentIndex(
    revision3VoiceContentIndexFixture(
      existingSlotCandidateCount: 1,
      existingSlotHasSelectedTake: true,
    ),
  );
  return Revision3VoiceTakeSelectionTechnicalPlan.forCheckpoint(
    catalog: catalog,
    lineId: revision3VoiceContentLineId,
    locale: 'de',
    selectedTakeId: null,
  );
}

AuthoringRevision3VoiceBuildResult _voiceBuildResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String output,
}) => AuthoringRevision3VoiceBuildResult.fromJson(
  <String, Object?>{
    'ok': true,
    'outcome': 'built',
    'basis_head_json': head.canonicalJson,
    'project_id': projectId,
    'project_revision': projectRevision,
    'output': output,
    'edit_count': 1,
    'file_count': 3,
    'bundle_bytes': 1234,
    'bundle_sha256': 'd' * 64,
    'build_authority': 'generation_sealed_existing_member_bundle_v1',
    'deployment_status': 'not_performed',
  },
  expectedHead: head,
  expectedProjectJson: revision3VoiceFixtureBuildReadyProjectJson(
    projectId: projectId,
    projectRevision: projectRevision,
  ),
  expectedOutput: output,
);

AuthoringWorkingHead _head(int value) => AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{
      'byte_len': value + 1,
      'sha256': value.toRadixString(16).padLeft(64, '0'),
    },
  }),
);

Revision3ContentIndex _contentIndex({
  required String projectId,
  required int revision,
}) => Revision3ContentIndex.fromJsonObject(<String, Object?>{
  'schema_revision': 1,
  'project_id': projectId,
  'project_revision': revision,
  'project_name': 'Controller content',
  'project_version': '1.0.0',
  'project_author': 'tests',
  'target': <String, Object?>{
    'executable': <String, Object?>{
      'byte_len': 1,
      'sha256': List<String>.filled(64, '5').join(),
    },
  },
  'authoring_locales': <Object?>[],
  'entity_counts': <String, Object?>{},
  'entities': <Object?>[],
  'assets': <Object?>[],
});

AuthoringRevision3DataAssetPackageIndexResult
_controllerDataAssetPackageIndexResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
}) {
  final indexJson = jsonEncode(<String, Object?>{
    'status': 'complete_index',
    'physical_chunk_count': 1,
    'winning_export_bundle_count': 1,
    'directory_indexed_export_bundle_count': 1,
    'out_of_scope_export_bundle_count': 0,
    'candidates': <Object?>[
      <String, Object?>{
        'target_path': '/Game/Characters/DA_Asghan',
        'package_id_hex': '0123456789abcdef',
      },
    ],
    'partial_reasons': <Object?>[],
  });
  final indexBytes = utf8.encode(indexJson);
  Map<String, Object?> seal(int byteLength, String sha256) => <String, Object?>{
    'byte_len': byteLength,
    'sha256': sha256,
  };
  return AuthoringRevision3DataAssetPackageIndexResult.fromJson(
    <String, Object?>{
      'authority_status': 'not_granted',
      'build_status': 'not_evaluated',
      'candidate_count': 1,
      'content_status': 'metadata_candidates_only',
      'export_bundle_payload_status': 'not_read',
      'head_json': head.canonicalJson,
      'mount_inventory_entry_count': 2,
      'mount_inventory_seal': seal(80, 'b' * 64),
      'mutation_status': 'not_supported',
      'ok': true,
      'outcome': 'audit_only',
      'package_index_json': indexJson,
      'package_index_seal': seal(
        indexBytes.length,
        crypto.sha256.convert(indexBytes).toString(),
      ),
      'package_index_status': 'complete_index',
      'project_id': projectId,
      'project_revision': projectRevision,
      'publication_status': 'not_supported',
      'runtime_status': 'runtime_unqualified',
      'scope': 'installed_dataasset_package_candidates_only',
      'source_snapshot_seal': seal(120, 'c' * 64),
      'target_executable_seal': seal(171698176, 'd' * 64),
    },
    expectedHead: head,
  );
}

AuthoringRevision3QuestSourceInspectionResult _controllerQuestInspectionResult({
  required AuthoringWorkingHead head,
  required String projectId,
  required int projectRevision,
  required String questId,
}) {
  const moduleId = '72727272727272727272727272727272';
  const source = '''class UQuest_GoreControllerInspection : UQuest
{
    void OnStart() {}
}
''';
  final sourceBytes = utf8.encode(source);
  final sourceSha = crypto.sha256.convert(sourceBytes).toString();
  final projectBytes = utf8.encode(
    'controller Quest inspection $projectId@$projectRevision',
  );
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
        'module_namespace': 'GoreMods.Quests.ControllerInspection',
        'module_relative_path': 'GoreMods/Quests/ControllerInspection.as',
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

AuthoringRevision3DataAssetStage _dataAssetStage() {
  final fixture = revision3DataAssetNativeGoldenFixture();
  return AuthoringRevision3DataAssetStageListResult.fromJson(
    fixture.listResponse(),
    expectedHead: fixture.stagedHead,
  ).stages.single;
}

DataAssetSemanticEditIntent _dataAssetSemanticIntent() {
  final inspection = DataAssetInspection.fromJson(
    validDataAssetInspectionResponse(),
  );
  return DataAssetSemanticValueEditor.fromLeaf(
        inspection.exports.single.leaves.single,
      )
      .previewScalar(
        extractReceiptPath: r'C:\proof\extract-receipt.v2.json',
        expectedTargetPath: '/Game/TestAsset',
        value: '2',
      )
      .intent;
}

Revision3QuestDraftAuthoringInput _questInput() =>
    Revision3QuestDraftAuthoringInput(
      parentCatalogId: 'chapter-one',
      giverCatalogId: 'asghan',
      title: 'Find Homer',
      description: 'Homer vanished near the old gate.',
      objectiveTitle: 'Ask Asghan about Homer',
    );

Revision3NpcDraftAuthoringInput _npcInput() => Revision3NpcDraftAuthoringInput(
  parentCatalogId: 'g1r:npc:om_grd_asghan_263',
  displayName: 'North Gate Guard',
);
