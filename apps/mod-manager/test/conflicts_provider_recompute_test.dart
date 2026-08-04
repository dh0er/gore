import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/library/domain/conflicts_provider.dart';
import 'package:gore_manager/library/domain/library_notifier.dart';
import 'package:gore_manager/library/domain/models.dart';

/// A stateful core service whose `mgr_library_list` reports a SINGLE mod (id
/// `m1`) whose components can be swapped between refreshes — simulating a
/// same-id re-import (an update) that changes the mod's content but not the
/// loadout. Records every command so a test can count `mgr_analyze` calls.
class _StatefulFake implements GoreCoreFfiService {
  _StatefulFake(this._components);

  /// The components `mgr_library_list` currently reports for mod `m1`.
  List<Map<String, Object?>> _components;
  final calls = <({String command, Map<String, Object?> payload})>[];

  /// Swap `m1`'s components (the same-id update). The next `mgr_library_list`
  /// reports the new content; the loadout (id + enabled) is unchanged.
  void setComponents(List<Map<String, Object?>> components) {
    _components = components;
  }

  int get analyzeCount => calls.where((c) => c.command == 'mgr_analyze').length;

  @override
  bool get isAvailable => true;

  @override
  String get description => 'stateful-fake';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    calls.add((command: command, payload: payload));
    switch (command) {
      case 'mgr_library_list':
        return {
          'ok': true,
          'mods': [
            {
              'id': 'm1',
              'kind': 'goremod',
              'name': 'M1',
              'version': '',
              'author': '',
              'imported_at': '',
              'source': '',
              'components': _components,
            },
          ],
          // Loadout: m1 enabled. Stable across refreshes (only content changes).
          'loadout': {
            'format': 1,
            'entries': [
              {'id': 'm1', 'enabled': true},
            ],
          },
        };
      case 'mgr_analyze':
        return {'ok': true, 'conflicts': <Object?>[]};
      default:
        return {'ok': true};
    }
  }
}

/// A single loc_patch component whose target set we can vary.
List<Map<String, Object?>> _loc(List<String> targets) => [
      {'type': 'loc_patch', 'rel': 'loc/edits.json', 'targets': targets},
    ];

List<Map<String, Object?>> _lua({required bool opaque}) => [
  {
    'type': 'ue4ss_lua',
    'name': 'Runtime',
    'rel': 'ue4ss/Runtime',
    'targets': const ['A.Value'],
    'opaque': opaque,
  },
];

/// Drive the container until every pending microtask/future settles.
Future<void> _settle() => Future<void>.delayed(Duration.zero);

void main() {
  test('conflictsProvider re-analyzes when a mod content changes (same loadout)',
      () async {
    final fake = _StatefulFake(_loc(const ['itfo_cheese|german']));
    final container = ProviderContainer(overrides: [
      coreServiceProvider.overrideWithValue(fake),
    ]);
    addTearDown(container.dispose);

    // Hold live subscriptions so neither provider is auto-disposed between
    // reads — otherwise a bare `read` would recompute from scratch each time
    // (re-running analyze regardless of the key) and the test would prove
    // nothing about the key. With a listener, analyze only re-runs when the
    // watched key actually changes.
    container.listen(libraryProvider, (_, _) {});
    container.listen(conflictsProvider, (_, _) {});

    // Let the initial library refresh + first analyze settle.
    await container.read(conflictsProvider.future);
    await container.read(libraryProvider.notifier).refresh();
    await _settle();
    await container.read(conflictsProvider.future);
    final baseline = fake.analyzeCount;
    expect(baseline, greaterThanOrEqualTo(1), reason: 'analyze ran at least once');

    // Same-id UPDATE: change m1's component targets, keep the loadout identical.
    fake.setComponents(_loc(const ['itfo_apple|german', 'itfo_bread|german']));
    await container.read(libraryProvider.notifier).refresh();
    await _settle();
    // The conflicts key now folds in the changed targets → analyze re-runs.
    await container.read(conflictsProvider.future);

    expect(
      fake.analyzeCount,
      greaterThan(baseline),
      reason: 'a same-id content change must trigger a re-analyze',
    );
  });

  test('conflictsProvider does NOT re-analyze when nothing changed', () async {
    final fake = _StatefulFake(_loc(const ['itfo_cheese|german']));
    final container = ProviderContainer(overrides: [
      coreServiceProvider.overrideWithValue(fake),
    ]);
    addTearDown(container.dispose);

    container.listen(libraryProvider, (_, _) {});
    container.listen(conflictsProvider, (_, _) {});

    await container.read(conflictsProvider.future);
    await container.read(libraryProvider.notifier).refresh();
    await _settle();
    await container.read(conflictsProvider.future);
    final baseline = fake.analyzeCount;

    // A refresh that reloads the SAME content must not change the key, so the
    // cached conflicts stand (no new analyze).
    await container.read(libraryProvider.notifier).refresh();
    await _settle();
    await container.read(conflictsProvider.future);

    expect(
      fake.analyzeCount,
      baseline,
      reason: 'an unchanged library must not re-run analyze',
    );
  });

  test(
    'conflictsProvider re-analyzes when only UE4SS opacity changes',
    () async {
      final fake = _StatefulFake(_lua(opaque: false));
      final container = ProviderContainer(
        overrides: [coreServiceProvider.overrideWithValue(fake)],
      );
      addTearDown(container.dispose);
      container.listen(libraryProvider, (_, _) {});
      container.listen(conflictsProvider, (_, _) {});

      await container.read(conflictsProvider.future);
      await container.read(libraryProvider.notifier).refresh();
      await _settle();
      await container.read(conflictsProvider.future);
      final baseline = fake.analyzeCount;

      fake.setComponents(_lua(opaque: true));
      await container.read(libraryProvider.notifier).refresh();
      await _settle();
      await container.read(conflictsProvider.future);

      expect(fake.analyzeCount, greaterThan(baseline));
    },
  );

  test('unknown info advisory is ordered but has no winner', () {
    const conflict = ConflictView(
      kind: 'ue4ss_unknown',
      target: '<unknown>',
      modIds: ['later', 'earlier'],
      severity: 'info',
    );
    final chain = orderConflictChain(conflict, const ['earlier', 'later']);
    expect(chain.modIds, const ['earlier', 'later']);
    expect(chain.winnerId, isNull);
  });

  test('proven soft conflict keeps later-wins winner', () {
    const conflict = ConflictView(
      kind: 'cdo',
      target: 'A.Value',
      modIds: ['later', 'earlier'],
      severity: 'soft',
    );
    final chain = orderConflictChain(conflict, const ['earlier', 'later']);
    expect(chain.modIds, const ['earlier', 'later']);
    expect(chain.winnerId, 'later');
  });
}
