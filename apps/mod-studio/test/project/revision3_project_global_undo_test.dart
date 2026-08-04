import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/project/revision3_project_global_undo.dart';
import 'package:gore_mod/project/revision3_project_history.dart';

void main() {
  test('fresh exact history restores only the immediate predecessor', () async {
    var current = _checkpoint(7);
    final history = _history(currentRevision: 7, oldestRevision: 5);
    var loadCalls = 0;
    var confirmCalls = 0;
    var restoreCalls = 0;
    final coordinator = Revision3ProjectGlobalUndoCoordinator(
      readCurrentCheckpoint: () => current,
      loadHistory: (basis) async {
        loadCalls++;
        expect(basis.sameAs(current), isTrue);
        return history;
      },
      confirm: (plan) async {
        confirmCalls++;
        expect(plan.basis.sameAs(current), isTrue);
        expect(identical(plan.history, history), isTrue);
        expect(plan.target.projectRevision, 6);
        expect(plan.nextRevision, 8);
        return true;
      },
      restore: (basis, expectedHistory, target) async {
        restoreCalls++;
        expect(basis.sameAs(current), isTrue);
        expect(identical(expectedHistory, history), isTrue);
        expect(identical(target, history.immediatePrevious), isTrue);
        final publication = _publication(history, target);
        current = _checkpoint(8, head: publication.head);
        return publication;
      },
    );
    addTearDown(coordinator.dispose);

    final result = await coordinator.undo();

    expect(result.outcome, Revision3ProjectGlobalUndoOutcome.restored);
    expect(result.publication?.restoredFromRevision, 6);
    expect((loadCalls, confirmCalls, restoreCalls), (1, 1, 1));
    expect(coordinator.isBusy, isFalse);
  });

  test(
    'a root checkpoint reports nothing to undo without confirmation',
    () async {
      final current = _checkpoint(0);
      var confirms = 0;
      var restores = 0;
      final coordinator = Revision3ProjectGlobalUndoCoordinator(
        readCurrentCheckpoint: () => current,
        loadHistory: (_) async =>
            _history(currentRevision: 0, oldestRevision: 0),
        confirm: (_) async {
          confirms++;
          return true;
        },
        restore: (_, _, _) async {
          restores++;
          throw StateError('not called');
        },
      );
      addTearDown(coordinator.dispose);

      final result = await coordinator.undo();

      expect(result.outcome, Revision3ProjectGlobalUndoOutcome.nothingToUndo);
      expect((confirms, restores), (0, 0));
    },
  );

  test(
    'cancelling preserves the exact checkpoint and never restores',
    () async {
      final current = _checkpoint(3);
      final history = _history(currentRevision: 3, oldestRevision: 2);
      var restores = 0;
      final coordinator = Revision3ProjectGlobalUndoCoordinator(
        readCurrentCheckpoint: () => current,
        loadHistory: (_) async => history,
        confirm: (_) async => false,
        restore: (_, _, _) async {
          restores++;
          throw StateError('not called');
        },
      );
      addTearDown(coordinator.dispose);

      final result = await coordinator.undo();

      expect(result.outcome, Revision3ProjectGlobalUndoOutcome.cancelled);
      expect(restores, 0);
    },
  );

  for (final gate in ['dirty edit', 'recovery', 'host busy']) {
    test('$gate gate fails closed before loading History', () async {
      var loads = 0;
      final coordinator = Revision3ProjectGlobalUndoCoordinator(
        readCurrentCheckpoint: () => null,
        loadHistory: (_) async {
          loads++;
          throw StateError('not called');
        },
        confirm: (_) async => true,
        restore: (_, _, _) async => throw StateError('not called'),
      );
      addTearDown(coordinator.dispose);

      final result = await coordinator.undo();

      expect(result.outcome, Revision3ProjectGlobalUndoOutcome.unavailable);
      expect(loads, 0);
    });
  }

  test(
    'head drift during History load prevents confirmation and restore',
    () async {
      var current = _checkpoint(4);
      final pending = Completer<Revision3ProjectHistorySnapshot>();
      var confirms = 0;
      var restores = 0;
      final coordinator = Revision3ProjectGlobalUndoCoordinator(
        readCurrentCheckpoint: () => current,
        loadHistory: (_) => pending.future,
        confirm: (_) async {
          confirms++;
          return true;
        },
        restore: (_, _, _) async {
          restores++;
          throw StateError('not called');
        },
      );
      addTearDown(coordinator.dispose);

      final operation = coordinator.undo();
      current = _checkpoint(5);
      pending.complete(_history(currentRevision: 4, oldestRevision: 3));
      final result = await operation;

      expect(result.outcome, Revision3ProjectGlobalUndoOutcome.stale);
      expect((confirms, restores), (0, 0));
    },
  );

  test('dirty or head drift after confirmation cannot enter restore', () async {
    Revision3ProjectGlobalUndoCheckpoint? current = _checkpoint(4);
    final confirmation = Completer<bool>();
    var restores = 0;
    final coordinator = Revision3ProjectGlobalUndoCoordinator(
      readCurrentCheckpoint: () => current,
      loadHistory: (_) async => _history(currentRevision: 4, oldestRevision: 3),
      confirm: (_) => confirmation.future,
      restore: (_, _, _) async {
        restores++;
        throw StateError('not called');
      },
    );
    addTearDown(coordinator.dispose);

    final operation = coordinator.undo();
    await Future<void>.delayed(Duration.zero);
    current = null;
    confirmation.complete(true);
    final result = await operation;

    expect(result.outcome, Revision3ProjectGlobalUndoOutcome.stale);
    expect(restores, 0);
  });

  test('a second invocation is refused while the first is in flight', () async {
    final current = _checkpoint(2);
    final pending = Completer<Revision3ProjectHistorySnapshot>();
    final coordinator = Revision3ProjectGlobalUndoCoordinator(
      readCurrentCheckpoint: () => current,
      loadHistory: (_) => pending.future,
      confirm: (_) async => false,
      restore: (_, _, _) async => throw StateError('not called'),
    );
    addTearDown(coordinator.dispose);

    final first = coordinator.undo();
    final second = await coordinator.undo();
    expect(second.outcome, Revision3ProjectGlobalUndoOutcome.busy);

    pending.complete(_history(currentRevision: 2, oldestRevision: 1));
    expect((await first).outcome, Revision3ProjectGlobalUndoOutcome.cancelled);
  });

  test(
    'disposing during a late History read suppresses all continuations',
    () async {
      final current = _checkpoint(2);
      final pending = Completer<Revision3ProjectHistorySnapshot>();
      var confirms = 0;
      var restores = 0;
      final coordinator = Revision3ProjectGlobalUndoCoordinator(
        readCurrentCheckpoint: () => current,
        loadHistory: (_) => pending.future,
        confirm: (_) async {
          confirms++;
          return true;
        },
        restore: (_, _, _) async {
          restores++;
          throw StateError('not called');
        },
      );

      final operation = coordinator.undo();
      coordinator.dispose();
      pending.complete(_history(currentRevision: 2, oldestRevision: 1));
      final result = await operation;

      expect(result.outcome, Revision3ProjectGlobalUndoOutcome.superseded);
      expect((confirms, restores), (0, 0));
      expect(
        (await coordinator.undo()).outcome,
        Revision3ProjectGlobalUndoOutcome.unavailable,
      );
    },
  );

  test(
    'a successful restore finishing after disposal is not presented',
    () async {
      var current = _checkpoint(3);
      final history = _history(currentRevision: 3, oldestRevision: 2);
      final pending = Completer<Revision3ProjectHistoryRestorePublication>();
      final restoreEntered = Completer<void>();
      final coordinator = Revision3ProjectGlobalUndoCoordinator(
        readCurrentCheckpoint: () => current,
        loadHistory: (_) async => history,
        confirm: (_) async => true,
        restore: (_, _, target) {
          restoreEntered.complete();
          return pending.future;
        },
      );

      final operation = coordinator.undo();
      await restoreEntered.future;
      final publication = _publication(history, history.entries[1]);
      current = _checkpoint(4, head: publication.head);
      coordinator.dispose();
      pending.complete(publication);

      expect(
        (await operation).outcome,
        Revision3ProjectGlobalUndoOutcome.superseded,
      );
    },
  );

  test(
    'mismatched publication is rejected instead of widening success',
    () async {
      final current = _checkpoint(5);
      final history = _history(currentRevision: 5, oldestRevision: 4);
      final coordinator = Revision3ProjectGlobalUndoCoordinator(
        readCurrentCheckpoint: () => current,
        loadHistory: (_) async => history,
        confirm: (_) async => true,
        restore: (_, _, target) async =>
            Revision3ProjectHistoryRestorePublication(
              previousHead: history.basisHead,
              head: _head(99),
              projectId: history.projectId,
              previousProjectRevision: 5,
              projectRevision: 6,
              restoredFromHead: target.head,
              restoredFromRevision: 3,
            ),
      );
      addTearDown(coordinator.dispose);

      await expectLater(
        coordinator.undo(),
        throwsA(isA<Revision3ProjectGlobalUndoPublicationMismatch>()),
      );
    },
  );
}

const _projectId = '11111111111111111111111111111111';
const _root = r'C:\mods\story.goreproj';

Revision3ProjectGlobalUndoCheckpoint _checkpoint(
  int revision, {
  AuthoringWorkingHead? head,
}) => Revision3ProjectGlobalUndoCheckpoint(
  root: _root,
  projectId: _projectId,
  projectRevision: revision,
  head: head ?? _head(revision),
);

Revision3ProjectHistorySnapshot _history({
  required int currentRevision,
  required int oldestRevision,
}) {
  final entries = <Revision3ProjectHistoryEntry>[
    for (var revision = currentRevision; revision >= oldestRevision; revision--)
      Revision3ProjectHistoryEntry(
        head: _head(revision),
        projectId: _projectId,
        projectRevision: revision,
        isCurrent: revision == currentRevision,
      ),
  ];
  return Revision3ProjectHistorySnapshot(
    basisHead: entries.first.head,
    projectId: _projectId,
    currentRevision: currentRevision,
    entries: entries,
    historyTruncated: false,
  );
}

Revision3ProjectHistoryRestorePublication _publication(
  Revision3ProjectHistorySnapshot history,
  Revision3ProjectHistoryEntry target,
) => Revision3ProjectHistoryRestorePublication(
  previousHead: history.basisHead,
  head: _head(history.currentRevision + 101),
  projectId: history.projectId,
  previousProjectRevision: history.currentRevision,
  projectRevision: history.currentRevision + 1,
  restoredFromHead: target.head,
  restoredFromRevision: target.projectRevision,
);

AuthoringWorkingHead _head(int tag) => AuthoringWorkingHead.fromCanonicalJson(
  '{"store_format":1,"snapshot":{"byte_len":${tag + 1},"sha256":"${(tag + 1).toRadixString(16).padLeft(64, '0')}"}}',
);
