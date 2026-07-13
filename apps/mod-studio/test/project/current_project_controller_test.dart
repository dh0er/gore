import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/current_project_controller.dart';
import 'package:gore_mod/project/project_controller.dart';

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

final class _FakeManagedLease implements ManagedRevision3CurrentProjectLease {
  _FakeManagedLease({
    required this.root,
    required this.projectIdValue,
    required this.projectRevision,
    required this.head,
    this.projectIdError,
    this.onVerify,
    this.closeFailuresRemaining = 0,
  });

  @override
  final Directory root;
  final String projectIdValue;
  final Object? projectIdError;
  @override
  final int projectRevision;
  @override
  final AuthoringWorkingHead head;
  final _VerifyHook? onVerify;
  int closeFailuresRemaining;
  bool requiresReopenValue = false;
  int verifyCalls = 0;
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
  Future<void> close() async {
    closeCalls++;
    if (closeFailuresRemaining > 0) {
      closeFailuresRemaining--;
      throw StateError('injected close failure');
    }
  }
}

AuthoringWorkingHead _head(int value) => AuthoringWorkingHead.fromCanonicalJson(
  jsonEncode(<String, Object?>{
    'store_format': 1,
    'snapshot': <String, Object?>{
      'byte_len': value + 1,
      'sha256': value.toRadixString(16).padLeft(64, '0'),
    },
  }),
);
