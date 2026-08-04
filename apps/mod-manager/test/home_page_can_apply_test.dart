import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/home_page.dart';
import 'package:gore_manager/library/domain/library_notifier.dart';
import 'package:gore_manager/library/domain/models.dart';

/// A LibraryState whose loadout carries the given (id, enabled) entries.
LibraryState _library(List<(String id, bool enabled)> entries) {
  return LibraryState(
    mods: [
      for (final e in entries)
        ModEntryMetaView(id: e.$1, kind: 'goremod', name: e.$1),
    ],
    loadout: LoadoutView(
      entries: [
        for (final e in entries) LoadoutEntryView(id: e.$1, enabled: e.$2),
      ],
    ),
  );
}

ManagerStatusView _status(String state) =>
    ManagerStatusView.fromJson({'state': state});

void main() {
  group('canApply gate', () {
    final oneEnabled = _library([('mod-a', true)]);
    final oneDisabled = _library([('mod-a', false)]);
    final empty = _library(const []);

    test('nothing_deployed + >=1 enabled mod + game path set -> ENABLED', () {
      // The regression: the first-ever deploy must be reachable.
      expect(
        canApply(_status('nothing_deployed'), oneEnabled, true, false, false),
        isTrue,
      );
    });

    test('nothing_deployed + 0 enabled mods -> DISABLED', () {
      expect(
        canApply(_status('nothing_deployed'), oneDisabled, true, false, false),
        isFalse,
      );
      expect(
        canApply(_status('nothing_deployed'), empty, true, false, false),
        isFalse,
      );
    });

    test('in_sync -> DISABLED even with enabled mods', () {
      expect(
        canApply(_status('in_sync'), oneEnabled, true, false, false),
        isFalse,
      );
    });

    test('changes_pending -> ENABLED', () {
      expect(
        canApply(_status('changes_pending'), oneEnabled, true, false, false),
        isTrue,
      );
      // Target-vs-deployed drift doesn't hinge on the local enabled count.
      expect(
        canApply(_status('changes_pending'), empty, true, false, false),
        isTrue,
      );
    });

    test('game_updated -> ENABLED', () {
      expect(
        canApply(_status('game_updated'), oneEnabled, true, false, false),
        isTrue,
      );
    });

    test('studio_deploy_active -> DISABLED (take-over path, not Apply)', () {
      expect(
        canApply(
            _status('studio_deploy_active'), oneEnabled, true, false, false),
        isFalse,
      );
    });

    test('recovery_required -> DISABLED until recovery undeploy', () {
      expect(
        canApply(
          _status('recovery_required'),
          oneEnabled,
          true,
          false,
          false,
        ),
        isFalse,
      );
    });

    test('studioActive flag -> DISABLED even when status is ChangesPending', () {
      // A prior apply was blocked by an active studio deploy; the status may not
      // have caught up (e.g. still changes_pending), but Apply must stay off.
      expect(
        canApply(_status('changes_pending'), oneEnabled, true, false, true),
        isFalse,
      );
    });

    test('null status -> DISABLED', () {
      expect(canApply(null, oneEnabled, true, false, false), isFalse);
    });

    test('no game path -> DISABLED regardless of status/loadout', () {
      expect(
        canApply(_status('changes_pending'), oneEnabled, false, false, false),
        isFalse,
      );
      expect(
        canApply(_status('nothing_deployed'), oneEnabled, false, false, false),
        isFalse,
      );
    });

    test('busy -> DISABLED regardless of status/loadout', () {
      expect(
        canApply(_status('changes_pending'), oneEnabled, true, true, false),
        isFalse,
      );
      expect(
        canApply(_status('nothing_deployed'), oneEnabled, true, true, false),
        isFalse,
      );
    });

    test('library.busy -> DISABLED even when status is ChangesPending', () {
      // A toggle/reorder sets library.busy while it persists the loadout via
      // mgr_set_loadout; Apply must wait so mgr_apply can't read a stale
      // on-disk loadout. status.busy is false here — only library.busy blocks.
      final busyLibrary = _library([('mod-a', true)]).copyWith(busy: true);
      expect(
        canApply(_status('changes_pending'), busyLibrary, true, false, false),
        isFalse,
      );
    });
  });
}
