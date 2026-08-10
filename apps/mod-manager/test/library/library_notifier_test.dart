import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/mgr_ffi.dart';
import 'package:gore_manager/library/domain/library_notifier.dart';

/// A library_list response with the given mods (id->kind) and loadout entries.
Map<String, Object?> _libraryList({
  required List<(String id, bool enabled)> loadout,
  List<String>? mods,
}) {
  final ids = mods ?? [for (final e in loadout) e.$1];
  return {
    'ok': true,
    'mods': [
      for (final id in ids)
        {'id': id, 'kind': 'goremod', 'name': id.toUpperCase()},
    ],
    'loadout': {
      'format': 1,
      'entries': [
        for (final e in loadout) {'id': e.$1, 'enabled': e.$2},
      ],
    },
  };
}

class _BlockingRemoveCore implements GoreCoreFfiService {
  final removeStarted = Completer<void>();
  final releaseRemove = Completer<void>();
  final List<({String command, Map<String, Object?> payload})> calls = [];
  bool removed = false;

  @override
  bool get isAvailable => true;

  @override
  String get description => 'blocking-remove-test';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    calls.add((command: command, payload: payload));
    switch (command) {
      case 'mgr_remove':
        removeStarted.complete();
        await releaseRemove.future;
        removed = true;
        return {'ok': true, 'removed': true};
      case 'mgr_library_list':
        return _libraryList(
          loadout: removed
              ? [('mod-b', true)]
              : [('mod-a', true), ('mod-b', true)],
        );
      case 'mgr_set_loadout':
        return {'ok': true};
      default:
        return {
          'ok': false,
          'error': {'code': 'UNKNOWN', 'message': 'unknown command'},
        };
    }
  }
}

/// Simulates a native import that publishes its library entry and only then
/// reports an error. The following authoritative reload can independently be
/// failed to exercise the fail-closed boundary.
class _PartialImportCore implements GoreCoreFfiService {
  bool imported = false;
  bool failReload = false;
  final List<({String command, Map<String, Object?> payload})> calls = [];

  @override
  bool get isAvailable => true;

  @override
  String get description => 'partial-import-test';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    calls.add((command: command, payload: payload));
    switch (command) {
      case 'mgr_import':
        imported = true;
        return {
          'ok': false,
          'error': {'code': 'IO', 'message': 'loadout follow-up failed'},
        };
      case 'mgr_library_list' when failReload:
        return {
          'ok': false,
          'error': {'code': 'IO', 'message': 'reload failed'},
        };
      case 'mgr_library_list':
        return _libraryList(
          loadout: imported
              ? [('mod-a', true), ('mod-new', false)]
              : [('mod-a', true)],
        );
      case 'mgr_set_loadout':
        return {'ok': true};
      default:
        return {
          'ok': false,
          'error': {'code': 'UNKNOWN', 'message': 'unknown command'},
        };
    }
  }
}

/// Drain the notifier's kicked-off refresh (or any pending async) so the
/// state settles before assertions.
Future<LibraryNotifier> _settled(FakeGoreCoreFfiService fake) async {
  final notifier = LibraryNotifier(MgrFfi(fake));
  await notifier.refresh();
  return notifier;
}

void main() {
  group('LibraryNotifier.refresh reconciliation', () {
    test(
      'new library mod missing from loadout is appended disabled at end',
      () async {
        // Loadout only knows mod-a; library has mod-a and a brand-new mod-b.
        final fake = FakeGoreCoreFfiService(
          responses: {
            'mgr_library_list': _libraryList(
              loadout: [('mod-a', true)],
              mods: ['mod-a', 'mod-b'],
            ),
            // Reconcile appends mod-b, so the notifier persists the result.
            'mgr_set_loadout': {'ok': true},
          },
        );
        final n = await _settled(fake);
        final entries = n.state.loadout.entries;
        expect(entries.map((e) => e.id), ['mod-a', 'mod-b']);
        expect(entries[0].enabled, isTrue);
        // Appended mod defaults to disabled.
        expect(entries[1].enabled, isFalse);
        expect(n.state.error, isNull);
        expect(n.state.busy, isFalse);
      },
    );

    test('loadout entry for a vanished mod is dropped', () async {
      // Loadout references mod-x which is no longer in the library.
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_library_list': _libraryList(
            loadout: [('mod-a', true), ('mod-x', true)],
            mods: ['mod-a'],
          ),
          // Reconcile drops the vanished mod-x, so the result is persisted.
          'mgr_set_loadout': {'ok': true},
        },
      );
      final n = await _settled(fake);
      expect(n.state.loadout.entries.map((e) => e.id), ['mod-a']);
    });

    test('a reconcile delta is persisted back via mgr_set_loadout', () async {
      // On-disk loadout has a stale entry (mod-x, gone from the library) and
      // is missing a present library mod (mod-b). Reconciliation drops mod-x
      // and appends mod-b disabled; that reconciled loadout must be written
      // back so the on-disk loadout matches the UI.
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_library_list': _libraryList(
            loadout: [('mod-a', true), ('mod-x', true)],
            mods: ['mod-a', 'mod-b'],
          ),
          'mgr_set_loadout': {'ok': true},
        },
      );
      final n = await _settled(fake);

      final setCall = fake.calls.firstWhere(
        (c) => c.command == 'mgr_set_loadout',
      );
      expect(setCall.payload, {
        'loadout': {
          'format': 1,
          'entries': [
            {'id': 'mod-a', 'enabled': true},
            {'id': 'mod-b', 'enabled': false},
          ],
        },
      });
      // The in-memory loadout matches what was persisted.
      expect(n.state.loadout.entries.map((e) => e.id), ['mod-a', 'mod-b']);
    });

    test(
      'an already-consistent loadout is not re-persisted (no loop)',
      () async {
        // Library and loadout already agree: reconcile is a no-op, so nothing
        // is written back and no set/refresh loop can start.
        final fake = FakeGoreCoreFfiService(
          responses: {
            'mgr_library_list': _libraryList(
              loadout: [('mod-a', true), ('mod-b', false)],
            ),
            'mgr_set_loadout': {'ok': true},
          },
        );
        final n = await _settled(fake);
        expect(
          fake.calls.where((c) => c.command == 'mgr_set_loadout'),
          isEmpty,
        );
        expect(n.state.loadout.entries.map((e) => e.id), ['mod-a', 'mod-b']);
      },
    );
  });

  group('LibraryNotifier.toggle', () {
    test('flips enabled and persists the full loadout in order', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_library_list': _libraryList(
            loadout: [('mod-a', true), ('mod-b', false)],
          ),
          'mgr_set_loadout': {'ok': true},
        },
      );
      final n = await _settled(fake);
      fake.calls.clear();

      await n.toggle('mod-a'); // true -> false

      // The set_loadout payload must carry both entries, in order, with
      // mod-a now disabled and mod-b untouched.
      final setCall = fake.calls.firstWhere(
        (c) => c.command == 'mgr_set_loadout',
      );
      expect(setCall.payload, {
        'loadout': {
          'format': 1,
          'entries': [
            {'id': 'mod-a', 'enabled': false},
            {'id': 'mod-b', 'enabled': false},
          ],
        },
      });
      // Refresh follows the set.
      expect(
        fake.calls.where((c) => c.command == 'mgr_library_list'),
        isNotEmpty,
      );
    });
  });

  group('LibraryNotifier.reorder', () {
    test('moves an entry down and saves the new order', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_library_list': _libraryList(
            loadout: [('mod-a', true), ('mod-b', true), ('mod-c', true)],
          ),
          'mgr_set_loadout': {'ok': true},
        },
      );
      final n = await _settled(fake);
      fake.calls.clear();

      // Classic ReorderableList convention: move index 0 to index 2 lands it
      // after removal at the end.
      await n.reorder(0, 3);

      final setCall = fake.calls.firstWhere(
        (c) => c.command == 'mgr_set_loadout',
      );
      final entries = (setCall.payload['loadout']! as Map)['entries']! as List;
      expect(
        [for (final e in entries) (e as Map)['id']],
        ['mod-b', 'mod-c', 'mod-a'],
      );
    });

    test('moves an entry up and saves the new order', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_library_list': _libraryList(
            loadout: [('mod-a', true), ('mod-b', true), ('mod-c', true)],
          ),
          'mgr_set_loadout': {'ok': true},
        },
      );
      final n = await _settled(fake);
      fake.calls.clear();

      await n.reorder(2, 0); // mod-c to the front

      final setCall = fake.calls.firstWhere(
        (c) => c.command == 'mgr_set_loadout',
      );
      final entries = (setCall.payload['loadout']! as Map)['entries']! as List;
      expect(
        [for (final e in entries) (e as Map)['id']],
        ['mod-c', 'mod-a', 'mod-b'],
      );
    });
  });

  group('LibraryNotifier.import', () {
    test('imports then refreshes', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_import': {
            'ok': true,
            'entry': {'id': 'mod-new', 'kind': 'foreign_pak', 'name': 'New'},
          },
          'mgr_library_list': _libraryList(
            loadout: [('mod-a', true)],
            mods: ['mod-a', 'mod-new'],
          ),
          // The post-import refresh appends mod-new, so it is persisted.
          'mgr_set_loadout': {'ok': true},
        },
      );
      final n = await _settled(fake);
      fake.calls.clear();

      await n.import('D:/downloads/new.pak');

      expect(fake.calls[0].command, 'mgr_import');
      expect(fake.calls[0].payload, {'path': 'D:/downloads/new.pak'});
      // Followed by a refresh that surfaces the new mod, appended disabled.
      expect(fake.calls.any((c) => c.command == 'mgr_library_list'), isTrue);
      expect(n.state.mods.map((m) => m.id), contains('mod-new'));
      final newEntry = n.state.loadout.entries.firstWhere(
        (e) => e.id == 'mod-new',
      );
      expect(newEntry.enabled, isFalse);
    });

    test(
      'reloads authoritative truth after a partial native failure',
      () async {
        final core = _PartialImportCore();
        final notifier = LibraryNotifier(MgrFfi(core));
        await notifier.refresh();

        await notifier.import('D:/downloads/new.pak');

        expect(notifier.state.authoritative, isTrue);
        expect(notifier.state.mods.map((mod) => mod.id), contains('mod-new'));
        expect(notifier.state.error, contains('loadout follow-up failed'));
        expect(
          core.calls.where((call) => call.command == 'mgr_library_list'),
          hasLength(2),
        );
      },
    );

    test(
      'double failure clears stale state and refresh heals authority',
      () async {
        final core = _PartialImportCore();
        final notifier = LibraryNotifier(MgrFfi(core));
        await notifier.refresh();
        core.failReload = true;

        await notifier.import('D:/downloads/new.pak');

        expect(notifier.state.authoritative, isFalse);
        expect(notifier.state.mods, isEmpty);
        expect(notifier.state.loadout.entries, isEmpty);
        expect(notifier.state.error, contains('loadout follow-up failed'));

        final callsBeforeBlockedToggle = core.calls.length;
        await notifier.toggle('mod-a');
        expect(core.calls, hasLength(callsBeforeBlockedToggle));

        core.failReload = false;
        await notifier.refresh();
        expect(notifier.state.authoritative, isTrue);
        expect(notifier.state.mods.map((mod) => mod.id), contains('mod-new'));
        expect(notifier.state.error, isNull);
      },
    );
  });

  group('LibraryNotifier single-flight', () {
    test(
      'ignores a second library mutation while remove is in flight',
      () async {
        final core = _BlockingRemoveCore();
        final notifier = LibraryNotifier(MgrFfi(core));
        await notifier.refresh();
        core.calls.clear();

        final removing = notifier.remove('mod-a');
        await core.removeStarted.future;
        expect(notifier.state.busy, isTrue);

        // A fast second action must not reach native code while the destructive
        // library mutation still owns the source of truth.
        await notifier.toggle('mod-b');
        expect(
          core.calls.where((call) => call.command == 'mgr_set_loadout'),
          isEmpty,
        );

        core.releaseRemove.complete();
        await removing;
        expect(notifier.state.busy, isFalse);
        expect(
          core.calls
              .where(
                (call) =>
                    call.command == 'mgr_remove' ||
                    call.command == 'mgr_set_loadout',
              )
              .map((call) => call.command),
          ['mgr_remove'],
        );
      },
    );
  });

  group('LibraryNotifier errors', () {
    test('an FFI error lands in state.error and clears busy', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_library_list': {
            'ok': false,
            'error': {'code': 'IO', 'message': 'library unreadable'},
          },
        },
      );
      final n = LibraryNotifier(MgrFfi(fake));
      await n.refresh();
      expect(n.state.error, contains('library unreadable'));
      expect(n.state.busy, isFalse);
    });

    test('a later successful call clears the prior error', () async {
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_library_list': _libraryList(loadout: [('mod-a', true)]),
        },
      );
      final n = LibraryNotifier(MgrFfi(fake));
      // Seed an error via a failing refresh against an empty fake first.
      final failing = LibraryNotifier(
        MgrFfi(FakeGoreCoreFfiService(responses: const {})),
      );
      await failing.refresh();
      expect(failing.state.error, isNotNull);

      await n.refresh();
      expect(n.state.error, isNull);
      expect(n.state.mods, hasLength(1));
    });
  });
}
