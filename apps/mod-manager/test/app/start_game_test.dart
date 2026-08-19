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

Widget _app({required GameLauncher launcher}) => ProviderScope(
  overrides: [
    coreServiceProvider.overrideWithValue(_core()),
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

    test('is null when there is nothing to launch', () {
      expect(gameExecutableFor(null), isNull);
      expect(gameExecutableFor('   '), isNull);
      // A real install layout whose executable is missing must not be reported
      // as launchable; the button stays disabled instead of failing on click.
      expect(gameExecutableFor(_fakeInstall(withExe: false)), isNull);
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
