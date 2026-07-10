import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/shared_config.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/app/game_paths.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/home_page.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:gore_manager/library/domain/library_notifier.dart';
import 'package:path/path.dart' as p;

/// A settings store that starts empty; only theme/locale/scale live here now
/// (the game path moved to the shared config, see [_freshSharedConfig]).
class _EmptySettingsStore implements UiSettingsStore {
  UiSettings _current = const UiSettings();

  @override
  UiSettings read() => _current;

  @override
  void write(UiSettings settings) => _current = settings;
}

/// A shared config backed by its own temp file, optionally pre-seeded with a
/// game path. Isolated per call (fresh temp dir) so tests never observe state
/// left over by another test or a previous run — unlike the app's default
/// FLUTTER_TEST shared-config stub, which points at one fixed temp path.
SharedConfig _freshSharedConfig({String? gamePath}) {
  final dir = Directory.systemTemp.createTempSync('gm_status_test_cfg');
  addTearDown(() {
    if (dir.existsSync()) dir.deleteSync(recursive: true);
  });
  final config = SharedConfig(File(p.join(dir.path, 'config.json')));
  if (gamePath != null) config.setGamePath(gamePath);
  return config;
}

Widget _app(FakeGoreCoreFfiService fake, UiSettingsStore store, SharedConfig config) {
  return ProviderScope(
    overrides: [
      coreServiceProvider.overrideWithValue(fake),
      uiSettingsStoreProvider.overrideWithValue(store),
      sharedConfigProvider.overrideWithValue(config),
    ],
    child: MaterialApp(
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      home: const HomePage(),
    ),
  );
}

/// A minimal stateful core service: `mgr_set_loadout` actually updates the
/// loadout that `mgr_library_list` reports back, so a toggle produces a real
/// loadout delta the way the DLL would (a canned-response fake cannot). Records
/// every call for assertions.
class _StatefulFake implements GoreCoreFfiService {
  _StatefulFake(this._loadout);

  List<Map<String, Object?>> _loadout;
  final calls = <({String command, Map<String, Object?> payload})>[];

  static const _mods = [
    {
      'id': 'm1',
      'kind': 'goremod',
      'name': 'M1',
      'version': '',
      'author': '',
      'imported_at': '',
      'source': '',
      'components': <Object?>[],
    },
  ];

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
          'mods': _mods,
          'loadout': {'entries': _loadout},
        };
      case 'mgr_set_loadout':
        final lo = payload['loadout'] as Map<String, Object?>?;
        final entries = (lo?['entries'] as List?)?.cast<Map<String, Object?>>();
        if (entries != null) _loadout = entries;
        return {'ok': true};
      case 'mgr_analyze':
        return {'ok': true, 'conflicts': <Object?>[]};
      case 'mgr_status':
        return {
          'ok': true,
          'status': {'state': 'nothing_deployed'},
        };
      default:
        return {'ok': true};
    }
  }
}

Widget _appService(
    GoreCoreFfiService svc, UiSettingsStore store, SharedConfig config) {
  return ProviderScope(
    overrides: [
      coreServiceProvider.overrideWithValue(svc),
      uiSettingsStoreProvider.overrideWithValue(store),
      sharedConfigProvider.overrideWithValue(config),
    ],
    child: MaterialApp(
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      home: const HomePage(),
    ),
  );
}

void main() {
  testWidgets('a loadout edit re-queries deployment status', (tester) async {
    // Start with the game path already set so status refreshes hit the FFI.
    final store = _EmptySettingsStore();
    final config = _freshSharedConfig(
      gamePath: 'C:/games/gothic/G1R/Binaries/Win64/G1R-Win64-Shipping.exe',
    );
    final fake = _StatefulFake([
      {'id': 'm1', 'enabled': false},
    ]);
    await tester.pumpWidget(_appService(fake, store, config));
    await tester.pumpAndSettle();

    final before = fake.calls.where((c) => c.command == 'mgr_status').length;
    expect(before, greaterThanOrEqualTo(1), reason: 'startup refresh ran');

    // Flip the mod's enabled flag (persists the loadout).
    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    await container.read(libraryProvider.notifier).toggle('m1');
    await tester.pumpAndSettle();

    // The loadout changed, so status was re-queried.
    final after = fake.calls.where((c) => c.command == 'mgr_status').length;
    expect(after, greaterThan(before),
        reason: 'a loadout edit must refresh status');
  });

  testWidgets('changing the game exe path refreshes status for the new root',
      (tester) async {
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': {
          'ok': true,
          'mods': [],
          'loadout': {'entries': []},
        },
        'mgr_analyze': {'ok': true, 'conflicts': []},
        'mgr_status': {
          'ok': true,
          'status': {'state': 'nothing_deployed'},
        },
      },
    );
    await tester.pumpWidget(
        _app(fake, _EmptySettingsStore(), _freshSharedConfig()));
    await tester.pumpAndSettle();

    // Startup ran mgr_status with a null root -> the sentinel, no FFI call.
    expect(
      fake.calls.where((c) => c.command == 'mgr_status'),
      isEmpty,
    );

    // A path whose game root resolves purely by path shape (no real install).
    const exe = 'C:/games/gothic/G1R/Binaries/Win64/G1R-Win64-Shipping.exe';
    final expectedRoot = gameRootFromExe(exe);
    expect(expectedRoot, isNotNull);

    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    container.read(gameExePathProvider.notifier).set(exe);
    await tester.pumpAndSettle();

    // The path change triggered a status refresh with the new game root.
    final statusCall =
        fake.calls.firstWhere((c) => c.command == 'mgr_status');
    expect(statusCall.payload['game_root'], expectedRoot);
  });
}
