import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/project_controller.dart';
import 'package:gore_mod/project/revision3_content_index.dart';
import 'package:gore_mod/project/revision3_dataasset_authoring.dart';
import 'package:gore_mod/project/revision3_npc_authoring.dart';
import 'package:gore_mod/project/revision3_quest_authoring.dart';
import 'package:gore_mod/project/revision3_quest_outline_authoring.dart';
import 'package:gore_mod/project/revision3_voice_authoring.dart';

import '../support/revision3_dataasset_fixture.dart';
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

typedef _VerifyHook = FutureOr<void> Function(_FakeManagedLease lease);
typedef _ContentReadHook = FutureOr<void> Function(_FakeManagedLease lease);
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
typedef _DataAssetRemoveHook =
    FutureOr<Revision3DataAssetStageRemovalPublication> Function(
      _FakeManagedLease lease,
      String targetPath,
    );

final class _FakeManagedLease implements ManagedRevision3CurrentProjectLease {
  _FakeManagedLease({
    required this.root,
    required this.projectIdValue,
    required this.projectRevision,
    required this.head,
    this.projectIdError,
    this.onVerify,
    this.onNpcPublish,
    this.onQuestPublish,
    this.onQuestOutlinePublish,
    this.onVoicePublish,
    this.onVoiceTargetPublish,
    this.onVoiceBuild,
    this.onDataAssetPublish,
    this.onDataAssetSemanticPublish,
    this.onDataAssetRemove,
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
  final _NpcPublishHook? onNpcPublish;
  final _QuestPublishHook? onQuestPublish;
  final _QuestOutlinePublishHook? onQuestOutlinePublish;
  final _VoicePublishHook? onVoicePublish;
  final _VoiceTargetPublishHook? onVoiceTargetPublish;
  final _VoiceBuildHook? onVoiceBuild;
  final _DataAssetPublishHook? onDataAssetPublish;
  final _DataAssetSemanticPublishHook? onDataAssetSemanticPublish;
  final _DataAssetRemoveHook? onDataAssetRemove;
  final List<AuthoringRevision3DataAssetStage> dataAssetStages;
  final Revision3ContentIndex? contentIndex;
  _ContentReadHook? onContentRead;
  int closeFailuresRemaining;
  bool requiresReopenValue = false;
  int verifyCalls = 0;
  int contentReadCalls = 0;
  int npcPublishCalls = 0;
  int questPublishCalls = 0;
  int questOutlinePublishCalls = 0;
  int voicePublishCalls = 0;
  int voiceTargetPublishCalls = 0;
  int voiceBuildCalls = 0;
  int dataAssetListCalls = 0;
  int dataAssetPublishCalls = 0;
  int dataAssetSemanticPublishCalls = 0;
  int dataAssetRemoveCalls = 0;
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
