import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/game_launcher.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/app/game_paths.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/home_page.dart';
import 'package:gore_manager/library/domain/library_notifier.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:path/path.dart' as p;

const _exeTail = ['G1R', 'Binaries', 'Win64', 'G1R-Win64-Shipping.exe'];

/// A directory shaped like a real install, so [gameExecutableFor] resolves.
String _fakeInstall({bool withExe = true}) {
  final root = Directory.systemTemp.createTempSync('gore_game').path;
  final exe = p.joinAll([root, ..._exeTail]);
  Directory(p.dirname(exe)).createSync(recursive: true);
  if (withExe) File(exe).writeAsStringSync('');
  addTearDown(() {
    try {
      Directory(root).deleteSync(recursive: true);
    } catch (_) {
      // A leftover temp dir must never fail a test.
    }
  });
  return root;
}

FakeGoreCoreFfiService _core() => FakeGoreCoreFfiService(
  responses: {
    'mgr_library_list': {
      'ok': true,
      'mods': <Object?>[],
      'loadout': {'format': 1, 'entries': <Object?>[]},
    },
    'mgr_analyze': {'ok': true, 'conflicts': <Object?>[]},
  },
);

/// Delegates to the plain fake but can park one `mgr_library_list` call, so a
/// test can hold the library in `busy` while it taps.
class _ParkableCore implements GoreCoreFfiService {
  _ParkableCore(this._inner);

  final GoreCoreFfiService _inner;
  Completer<void>? park;

  @override
  bool get isAvailable => _inner.isAvailable;

  @override
  String get description => _inner.description;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    if (command == 'mgr_library_list' && park != null) {
      await park!.future;
      park = null;
    }
    return _inner.execute(command, payload: payload);
  }
}

Widget _app({required GameLauncher launcher, GoreCoreFfiService? core}) =>
    ProviderScope(
      overrides: [
        coreServiceProvider.overrideWithValue(core ?? _core()),
        gameLauncherProvider.overrideWithValue(launcher),
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

void _window(WidgetTester tester, Size size, {double textScale = 1}) {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  tester.platformDispatcher.textScaleFactorTestValue = textScale;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
}

final _startGame = find.byKey(const ValueKey('start-game-action'));

bool _enabled(WidgetTester tester) {
  final widget = tester.widget(_startGame);
  return switch (widget) {
    FilledButton(:final onPressed) => onPressed != null,
    IconButton(:final onPressed) => onPressed != null,
    _ => fail('unexpected start-game widget: ${widget.runtimeType}'),
  };
}

void main() {
  group('gameExecutableFor', () {
    test('resolves from the install root and from the exe itself', () {
      final root = _fakeInstall();
      final exe = p.joinAll([root, ..._exeTail]);
      // The shared config holds either form, so both must resolve.
      expect(gameExecutableFor(root), exe);
      expect(gameExecutableFor(exe), exe);
    });

    test('never launches a sibling executable', () {
      // The picker accepts any .exe, and Binaries/Win64 holds more than the
      // game; picking a helper there must still resolve to the game.
      final root = _fakeInstall();
      final helper = p.joinAll([
        root,
        'G1R',
        'Binaries',
        'Win64',
        'CrashReportClient.exe',
      ]);
      File(helper).writeAsStringSync('');
      expect(gameExecutableFor(helper), p.joinAll([root, ..._exeTail]));
    });

    test('is null when there is nothing to launch', () {
      expect(gameExecutableFor(null), isNull);
      expect(gameExecutableFor('   '), isNull);
      // A real install layout whose executable is missing must not be reported
      // as launchable; the button stays disabled instead of failing on click.
      expect(gameExecutableFor(_fakeInstall(withExe: false)), isNull);
      // An .exe that exists but sits outside an install resolves to no root.
      final stray = p.join(
        Directory.systemTemp.createTempSync('gore_stray').path,
        'Something.exe',
      );
      File(stray).writeAsStringSync('');
      expect(gameExecutableFor(stray), isNull);
    });
  });

  testWidgets('the tab row starts the resolved executable', (tester) async {
    _window(tester, const Size(1280, 800));
    final launched = <String>[];
    await tester.pumpWidget(
      _app(
        launcher: (exe) async {
          launched.add(exe);
          return true;
        },
      ),
    );
    await tester.pumpAndSettle();

    // Without a game path there is nothing to start.
    expect(_startGame, findsOneWidget);
    expect(_enabled(tester), isFalse);

    final root = _fakeInstall();
    ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    ).read(gameExePathProvider.notifier).set(root);
    await tester.pumpAndSettle();

    expect(_enabled(tester), isTrue);
    await tester.tap(_startGame);
    await tester.pumpAndSettle();

    expect(launched, [
      p.joinAll([root, ..._exeTail]),
    ]);
    expect(tester.takeException(), isNull);
  });

  testWidgets('the action survives a tab switch', (tester) async {
    _window(tester, const Size(1280, 800));
    await tester.pumpWidget(_app(launcher: (_) async => true));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    expect(_startGame, findsOneWidget);

    // It lives beside the tabs, not inside a tab, so leaving Mods keeps it.
    await tester.tap(find.text(l10n.tabSettings));
    await tester.pumpAndSettle();
    expect(_startGame, findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('a stale enabled callback cannot launch mid-operation', (
    tester,
  ) async {
    _window(tester, const Size(1280, 800));
    final launched = <String>[];
    final core = _ParkableCore(_core());
    await tester.pumpWidget(
      _app(
        core: core,
        launcher: (exe) async {
          launched.add(exe);
          return true;
        },
      ),
    );
    await tester.pumpAndSettle();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    container.read(gameExePathProvider.notifier).set(_fakeInstall());
    await tester.pumpAndSettle();
    expect(_enabled(tester), isTrue);

    // Grab the callback while the button is enabled. This is the one a tap
    // landing between that frame and the next still carries.
    final stale = tester.widget<FilledButton>(_startGame).onPressed!;

    // Park the next library read so an operation is genuinely in flight.
    core.park = Completer<void>();
    unawaited(container.read(libraryProvider.notifier).refresh());
    await tester.pump();
    expect(container.read(libraryProvider).busy, isTrue);
    expect(_enabled(tester), isFalse, reason: 'the next frame disables it');

    // Firing the stale callback must still not launch: the guard lives inside
    // the handler, not only in whether the button was drawn enabled.
    stale();
    await tester.pump();
    expect(launched, isEmpty, reason: 'must not launch mid-operation');

    core.park!.complete();
    await tester.pumpAndSettle();
    expect(launched, isEmpty);
    expect(tester.takeException(), isNull);
  });

  testWidgets('a launch failure is reported, not swallowed', (tester) async {
    _window(tester, const Size(1280, 800));
    await tester.pumpWidget(_app(launcher: (_) async => false));
    await tester.pumpAndSettle();

    ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    ).read(gameExePathProvider.notifier).set(_fakeInstall());
    await tester.pumpAndSettle();

    await tester.tap(_startGame);
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    expect(find.text(l10n.startGameFailed), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('a narrow window keeps the action, without its label', (
    tester,
  ) async {
    _window(tester, const Size(700, 460), textScale: 2);
    final semantics = tester.ensureSemantics();
    await tester.pumpWidget(_app(launcher: (_) async => true));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    // The TabBar shares this row and only scrolls, so the button shrinks
    // rather than pushing the tabs out of reach.
    expect(_startGame, findsOneWidget);
    expect(
      find.descendant(
        of: _startGame,
        matching: find.text(l10n.actionStartGame),
      ),
      findsNothing,
    );
    // The glyph alone must not be the whole story for assistive tech.
    expect(
      find.bySemanticsLabel(RegExp(RegExp.escape(l10n.actionStartGame))),
      findsWidgets,
    );
    expect(tester.takeException(), isNull);
    semantics.dispose();
  });
}
