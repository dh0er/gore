import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/shared_config.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/home_page.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:path/path.dart' as p;

const _exeA = 'C:/games/a/G1R/Binaries/Win64/G1R-Win64-Shipping.exe';
const _exeB = 'C:/games/b/G1R/Binaries/Win64/G1R-Win64-Shipping.exe';

class _SettingsStore implements UiSettingsStore {
  @override
  UiSettings read() => const UiSettings();

  @override
  void write(UiSettings settings) {}
}

SharedConfig _config(String? gamePath) {
  final directory = Directory.systemTemp.createTempSync('gm_preflight_home');
  addTearDown(() {
    if (directory.existsSync()) directory.deleteSync(recursive: true);
  });
  final config = SharedConfig(File(p.join(directory.path, 'config.json')));
  if (gamePath != null) config.setGamePath(gamePath);
  return config;
}

Map<String, Object?> _preflight({
  String? findingId,
  String findingState = 'problem',
  String findingAction = 'none',
  String findingDetail = 'Setup evidence needs attention.',
  String deploymentState = 'ok',
  String deploymentAction = 'none',
}) {
  const ids = [
    'game_root',
    'install',
    'loadout',
    'deployment',
    'install_mutation',
    'ue4ss',
    'write_access',
  ];
  return {
    'ok': true,
    'preflight': {
      'format': 1,
      'checks': [
        for (final id in ids)
          {
            'id': id,
            'state': id == findingId
                ? findingState
                : id == 'deployment'
                ? deploymentState
                : id == 'write_access'
                ? 'unverified'
                : 'ok',
            'code': id == findingId
                ? 'test_finding'
                : id == 'write_access'
                ? 'unverified_read_only'
                : 'ready',
            'action': id == findingId
                ? findingAction
                : id == 'deployment'
                ? deploymentAction
                : id == 'write_access'
                ? 'verify_during_apply'
                : 'none',
            'detail': id == findingId ? findingDetail : 'ready: $id',
            'items': <String>[],
          },
      ],
    },
  };
}

class _PreflightCore implements GoreCoreFfiService {
  _PreflightCore({
    Map<String, Object?>? preflight,
    this.status = const {'state': 'nothing_deployed'},
  }) : preflight = preflight ?? _preflight();

  Map<String, Object?> preflight;
  Map<String, Object?> status;
  bool blockNextPreflight = false;
  Completer<Map<String, Object?>>? blockedPreflight;
  final calls = <({String command, Map<String, Object?> payload})>[];

  @override
  String get description => 'home-preflight-test';

  @override
  bool get isAvailable => true;

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    calls.add((command: command, payload: payload));
    return switch (command) {
      'mgr_library_list' => {
        'ok': true,
        'mods': [
          {
            'id': 'alpha',
            'kind': 'goremod',
            'name': 'Alpha',
            'components': <Object?>[],
          },
        ],
        'loadout': {
          'format': 1,
          'entries': [
            {'id': 'alpha', 'enabled': true},
          ],
        },
      },
      'mgr_analyze' => {'ok': true, 'conflicts': <Object?>[]},
      'mgr_status' => {'ok': true, 'status': status},
      'mgr_preflight_v1' => await _runPreflight(),
      'mgr_apply' => {'ok': true, 'report': const {}},
      'mgr_undeploy_all' => {'ok': true, 'removed': true},
      _ => {'ok': true},
    };
  }

  Future<Map<String, Object?>> _runPreflight() {
    if (!blockNextPreflight) return Future.value(preflight);
    blockNextPreflight = false;
    final blocked = Completer<Map<String, Object?>>();
    blockedPreflight = blocked;
    return blocked.future;
  }

  int count(String command) =>
      calls.where((call) => call.command == command).length;
}

Widget _home(
  _PreflightCore core, {
  String? gamePath = _exeA,
  TextScaler textScaler = TextScaler.noScaling,
  Locale? locale,
}) {
  return ProviderScope(
    overrides: [
      coreServiceProvider.overrideWithValue(core),
      uiSettingsStoreProvider.overrideWithValue(_SettingsStore()),
      sharedConfigProvider.overrideWithValue(_config(gamePath)),
    ],
    child: MaterialApp(
      locale: locale,
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(context).copyWith(textScaler: textScaler),
        child: child!,
      ),
      home: const HomePage(),
    ),
  );
}

void main() {
  testWidgets('no path keeps the library visible and offers one Settings CTA', (
    tester,
  ) async {
    final core = _PreflightCore();
    await tester.pumpWidget(_home(core, gamePath: null));
    await tester.pumpAndSettle();

    expect(find.text('Alpha'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('preflight-no-game-path')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('preflight-settings-action')),
      findsOneWidget,
    );
    expect(core.count('mgr_preflight_v1'), 0);

    await tester.tap(find.byKey(const ValueKey('preflight-settings-action')));
    await tester.pumpAndSettle();
    expect(
      FocusManager.instance.primaryFocus?.debugLabel,
      'mod-manager-settings-game-path',
    );
  });

  testWidgets('invalid persisted selection is diagnosed natively unchanged', (
    tester,
  ) async {
    const invalid = 'C:/missing/not-the-game.exe';
    final core = _PreflightCore(
      preflight: _preflight(
        findingId: 'game_root',
        findingAction: 'select_game_root',
        findingDetail: 'The selected installation does not exist.',
      ),
    );

    await tester.pumpWidget(
      _home(core, gamePath: invalid, locale: const Locale('de')),
    );
    await tester.pumpAndSettle();

    final call = core.calls.singleWhere(
      (call) => call.command == 'mgr_preflight_v1',
    );
    expect(call.payload, {'game_root': invalid});
    expect(
      find.textContaining('Die Einrichtung benötigt Aufmerksamkeit.'),
      findsOneWidget,
    );
    expect(
      find.textContaining('The selected installation does not exist.'),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('preflight-settings-action')),
      findsOneWidget,
    );
  });

  testWidgets('late A snapshot never appears after selecting B', (
    tester,
  ) async {
    final core = _PreflightCore(
      preflight: _preflight(
        findingId: 'install',
        findingDetail: 'B is current.',
      ),
    )..blockNextPreflight = true;
    await tester.pumpWidget(_home(core));

    for (var i = 0; i < 20 && core.blockedPreflight == null; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }
    await tester.pump();
    expect(core.blockedPreflight, isNotNull);

    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    container.read(gameExePathProvider.notifier).set(_exeB);
    await tester.pump();
    core.blockedPreflight!.complete(
      _preflight(
        findingId: 'install',
        findingDetail: 'A is stale and must never appear.',
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.textContaining('A is stale and must never appear.'),
      findsNothing,
    );
    expect(find.textContaining('B is current.'), findsOneWidget);
    final roots = core.calls
        .where((call) => call.command == 'mgr_preflight_v1')
        .map((call) => call.payload['game_root'])
        .toList();
    expect(roots, [r'C:\games\a', r'C:\games\b']);
  });

  testWidgets(
    'neutral write evidence and deployment findings do not gate Apply',
    (tester) async {
      final core = _PreflightCore(
        preflight: _preflight(
          deploymentState: 'problem',
          deploymentAction: 'review_apply',
        ),
      );
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();

      expect(
        find.byKey(const ValueKey('preflight-setup-finding')),
        findsNothing,
      );
      final apply = tester.widget<FilledButton>(
        find.byKey(const ValueKey('apply-loadout-action')),
      );
      expect(apply.onPressed, isNotNull);
    },
  );

  testWidgets('future action has no side effect and offers Retry only', (
    tester,
  ) async {
    final core = _PreflightCore(
      preflight: _preflight(
        findingId: 'install',
        findingAction: 'format_drive',
        findingDetail: 'Future evidence.',
      ),
    );
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();

    expect(
      find.byKey(const ValueKey('preflight-retry-action')),
      findsOneWidget,
    );
    expect(find.byKey(const ValueKey('preflight-status-action')), findsNothing);
    expect(
      find.byKey(const ValueKey('preflight-settings-action')),
      findsNothing,
    );
    final beforeApply = core.count('mgr_apply');
    final beforePreflight = core.count('mgr_preflight_v1');
    await tester.tap(find.byKey(const ValueKey('preflight-retry-action')));
    await tester.pumpAndSettle();
    expect(core.count('mgr_apply'), beforeApply);
    expect(core.count('mgr_preflight_v1'), beforePreflight + 1);
  });

  testWidgets('unavailable diagnosis is friendly and Retry recovers', (
    tester,
  ) async {
    final core = _PreflightCore(
      preflight: {
        'ok': false,
        'error': {'code': 'INSPECTION_FAILED', 'message': 'raw native detail'},
      },
    );
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();

    expect(find.byKey(const ValueKey('preflight-unavailable')), findsOneWidget);
    expect(
      find.textContaining('Setup diagnosis is unavailable.'),
      findsOneWidget,
    );
    expect(find.textContaining('raw native detail'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('preflight-retry-action')),
      findsOneWidget,
    );

    core.preflight = _preflight();
    await tester.tap(find.byKey(const ValueKey('preflight-retry-action')));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('preflight-unavailable')), findsNothing);
  });

  testWidgets('failed keyboard Retry restores focus to the recreated action', (
    tester,
  ) async {
    final core = _PreflightCore(
      preflight: {
        'ok': false,
        'error': {'code': 'INSPECTION_FAILED', 'message': 'still unavailable'},
      },
    );
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();

    final retryFinder = find.byKey(const ValueKey('preflight-retry-action'));
    final retry = tester.widget<TextButton>(retryFinder);
    retry.focusNode!.requestFocus();
    await tester.pump();
    expect(retry.focusNode!.hasFocus, isTrue);

    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();

    expect(core.count('mgr_preflight_v1'), 2);
    expect(tester.widget<TextButton>(retryFinder).focusNode!.hasFocus, isTrue);
  });

  for (final transition in const [
    (
      name: 'Settings',
      action: 'select_game_root',
      key: 'preflight-settings-action',
    ),
    (
      name: 'Status',
      action: 'recover_deployment',
      key: 'preflight-status-action',
    ),
  ]) {
    testWidgets(
      'Retry does not restore focus when the result routes to ${transition.name}',
      (tester) async {
        final core = _PreflightCore(
          preflight: {
            'ok': false,
            'error': {
              'code': 'INSPECTION_FAILED',
              'message': 'temporarily unavailable',
            },
          },
        );
        await tester.pumpWidget(_home(core));
        await tester.pumpAndSettle();

        final retryFinder = find.byKey(
          const ValueKey('preflight-retry-action'),
        );
        final retry = tester.widget<TextButton>(retryFinder);
        retry.focusNode!.requestFocus();
        await tester.pump();
        core.preflight = _preflight(
          findingId: 'install',
          findingAction: transition.action,
        );

        await tester.sendKeyEvent(LogicalKeyboardKey.enter);
        await tester.pumpAndSettle();

        expect(find.byKey(ValueKey(transition.key)), findsOneWidget);
        expect(retry.focusNode!.hasFocus, isFalse);
      },
    );
  }

  testWidgets('Retry does not steal focus moved during its native read', (
    tester,
  ) async {
    final core = _PreflightCore(
      preflight: {
        'ok': false,
        'error': {'code': 'INSPECTION_FAILED', 'message': 'still unavailable'},
      },
    );
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();

    final retry = tester.widget<TextButton>(
      find.byKey(const ValueKey('preflight-retry-action')),
    );
    retry.focusNode!.requestFocus();
    await tester.pump();
    expect(retry.focusNode!.hasFocus, isTrue);
    core.blockNextPreflight = true;
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    for (var i = 0; i < 20 && core.blockedPreflight == null; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }
    expect(core.blockedPreflight, isNotNull);

    await tester.tap(find.text('Settings'));
    await tester.pumpAndSettle();
    final picker = tester.widget<OutlinedButton>(
      find.byKey(const ValueKey('settings-game-exe-pick')),
    );
    picker.focusNode!.requestFocus();
    await tester.pump();
    expect(picker.focusNode!.hasFocus, isTrue);

    core.blockedPreflight!.complete({
      'ok': false,
      'error': {'code': 'INSPECTION_FAILED', 'message': 'still unavailable'},
    });
    await tester.pumpAndSettle();

    expect(picker.focusNode!.hasFocus, isTrue);
    expect(retry.focusNode!.hasFocus, isFalse);
  });

  testWidgets(
    'diagnostic detail strips controls and truncates on rune bounds',
    (tester) async {
      final detail = '\u202E\u0085${List.filled(509, 'x').join()}😀TAIL';
      final core = _PreflightCore(
        preflight: _preflight(findingId: 'install', findingDetail: detail),
      );
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();

      final text = tester.widget<Text>(
        find.textContaining('Setup needs attention.'),
      );
      expect(text.data, isNot(contains('\u202E')));
      expect(text.data, isNot(contains('\u0085')));
      expect(text.data, contains('😀…'));
      expect(text.data, isNot(contains('TAIL')));
      expect(text.data, isNot(contains('\uFFFD')));
    },
  );

  testWidgets('Apply invalidates and refreshes its preflight snapshot', (
    tester,
  ) async {
    final core = _PreflightCore(
      preflight: _preflight(
        findingId: 'install_mutation',
        findingDetail: 'Old install evidence.',
      ),
    );
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();
    expect(core.count('mgr_preflight_v1'), 1);
    expect(find.textContaining('Old install evidence.'), findsOneWidget);

    core.preflight = _preflight();
    await tester.tap(find.byKey(const ValueKey('apply-loadout-action')));
    await tester.pumpAndSettle();

    expect(core.count('mgr_apply'), 1);
    expect(core.count('mgr_preflight_v1'), 2);
    expect(find.textContaining('Old install evidence.'), findsNothing);
  });

  testWidgets('Recovery invalidates and refreshes its preflight snapshot', (
    tester,
  ) async {
    final core = _PreflightCore(
      status: const {'state': 'recovery_required'},
      preflight: _preflight(
        findingId: 'install_mutation',
        findingAction: 'recover_deployment',
        findingDetail: 'Interrupted deployment needs review.',
      ),
    );
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();
    expect(core.count('mgr_preflight_v1'), 1);

    await tester.tap(find.byKey(const ValueKey('preflight-status-action')));
    await tester.pumpAndSettle();
    core.preflight = _preflight();
    core.status = const {'state': 'nothing_deployed'};
    await tester.tap(
      find.byKey(const ValueKey('status-details-action-recover')),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Recover'));
    await tester.pumpAndSettle();

    expect(core.count('mgr_undeploy_all'), 1);
    expect(core.count('mgr_preflight_v1'), 2);
    expect(
      find.textContaining('Interrupted deployment needs review.'),
      findsNothing,
    );
  });

  testWidgets('recovery hint opens existing status UI without mutating', (
    tester,
  ) async {
    final core = _PreflightCore(
      status: const {'state': 'recovery_required'},
      preflight: _preflight(
        findingId: 'install_mutation',
        findingAction: 'recover_deployment',
        findingDetail: 'Interrupted deployment needs review.',
      ),
    );
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('preflight-status-action')));
    await tester.pumpAndSettle();
    expect(find.textContaining('Deployment:'), findsOneWidget);
    expect(core.count('mgr_apply'), 0);
    expect(core.count('mgr_undeploy_all'), 0);
  });

  testWidgets('physical preflight read blocks every visible mutation lane', (
    tester,
  ) async {
    final core = _PreflightCore()..blockNextPreflight = true;
    await tester.pumpWidget(_home(core));
    for (var i = 0; i < 20 && core.blockedPreflight == null; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }
    await tester.pump();

    expect(
      tester
          .widget<OutlinedButton>(
            find.byKey(const ValueKey('import-mod-action')),
          )
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const ValueKey('apply-loadout-action')),
          )
          .onPressed,
      isNull,
    );
    expect(
      tester.widget<Checkbox>(find.byType(Checkbox).first).onChanged,
      isNull,
    );

    await tester.tap(find.text('Alpha'));
    await tester.pump();
    expect(
      tester
          .widget<OutlinedButton>(
            find.byKey(const ValueKey('remove-mod-action')),
          )
          .onPressed,
      isNull,
    );

    core.blockedPreflight!.complete(_preflight());
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const ValueKey('apply-loadout-action')),
          )
          .onPressed,
      isNotNull,
    );
  });

  testWidgets('compact 700x460 at 200 percent keeps finding actionable', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(700, 460);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final semantics = tester.ensureSemantics();

    final core = _PreflightCore(
      preflight: _preflight(
        findingId: 'ue4ss',
        findingAction: 'verify_ue4ss_proxy',
        findingDetail:
            'The configured UE4SS loader could not be verified for this enabled script mod.',
      ),
    );
    await tester.pumpWidget(
      _home(core, textScaler: const TextScaler.linear(2)),
    );
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    final action = find.byKey(const ValueKey('preflight-retry-action'));
    expect(action, findsOneWidget);
    expect(
      tester.getRect(action).overlaps(Offset.zero & tester.view.physicalSize),
      isTrue,
    );
    final node = tester.getSemantics(
      find.byKey(const ValueKey('preflight-setup-finding')),
    );
    expect(node.flagsCollection.isLiveRegion, isTrue);
    semantics.dispose();
  });
}
