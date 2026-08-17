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
import 'package:gore_manager/preflight/domain/preflight_notifier.dart';
import 'package:gore_manager/status/ui/status_details_dialog.dart';
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
  String findingCode = 'test_finding',
  String findingAction = 'none',
  String? findingActionToken,
  String findingDetail = 'Setup evidence needs attention.',
  List<String> findingItems = const [],
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
                ? findingCode
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
            if (id == findingId && findingActionToken != null)
              'action_token': findingActionToken,
            'detail': id == findingId ? findingDetail : 'ready: $id',
            'items': id == findingId ? findingItems : <String>[],
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
  String recoveryOutcome = 'recovered_to_pristine';
  String? recoveryError;
  bool blockNextPreflight = false;
  Completer<Map<String, Object?>>? blockedPreflight;
  bool blockNextRecovery = false;
  Completer<Map<String, Object?>>? blockedRecovery;
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
      'mgr_recover_install_v1' => await _runRecovery(),
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

  Future<Map<String, Object?>> _runRecovery() {
    final error = recoveryError;
    if (error != null) {
      return Future.value({
        'ok': false,
        'error': {'code': 'IO', 'message': error},
      });
    }
    if (!blockNextRecovery) {
      return Future.value({'ok': true, 'outcome': recoveryOutcome});
    }
    blockNextRecovery = false;
    final blocked = Completer<Map<String, Object?>>();
    blockedRecovery = blocked;
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
    (
      name: 'Recovery',
      action: 'recover_install',
      key: 'preflight-install-recovery-action',
    ),
    (
      name: 'Install wait',
      action: 'wait_for_install_mutation',
      key: 'preflight-install-recovery-action',
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
    final initialPreflight = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    ).read(preflightProvider);
    expect(initialPreflight.authoritative, isTrue);
    expect(initialPreflight.busy, isFalse);
    expect(initialPreflight.pending, isFalse);
    expect(
      initialPreflight.report?.primarySetupFinding?.rawAction,
      'recover_deployment',
    );
    expect(
      find.byKey(const ValueKey('preflight-status-action')),
      findsOneWidget,
    );

    await tester.tap(find.byKey(const ValueKey('preflight-status-action')));
    await tester.pumpAndSettle();
    final statusDialog = tester.widget<StatusDetailsDialog>(
      find.byType(StatusDetailsDialog),
    );
    expect(
      statusDialog.deploymentRecoveryGeneration,
      initialPreflight.generation,
    );
    core.preflight = _preflight();
    core.status = const {'state': 'nothing_deployed'};
    await tester.tap(
      find.byKey(const ValueKey('status-details-action-recover')),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, 'Recover'));
    await tester.pumpAndSettle();

    expect(core.count('mgr_undeploy_all'), 1);
    expect(core.count('mgr_recover_install_v1'), 0);
    expect(core.count('mgr_preflight_v1'), 2);
    expect(
      find.textContaining('Interrupted deployment needs review.'),
      findsNothing,
    );
  });

  testWidgets(
    'Manager recovery is the only recovery route for recovery-required status',
    (tester) async {
      final core = _PreflightCore(
        status: const {'state': 'recovery_required'},
        preflight: _preflight(
          findingId: 'install_mutation',
          findingCode: 'manager_mutation_recovery_required',
          findingAction: 'recover_manager_mutation',
          findingActionToken: 'guard-a-17',
        ),
      );
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();

      expect(
        find.byKey(const ValueKey('preflight-manager-recovery-action')),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const ValueKey('status-details-trigger')));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('status-details-action-recover')),
        findsNothing,
      );
      await tester.tap(
        find.byKey(const ValueKey('status-details-action-close')),
      );
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(const ValueKey('preflight-manager-recovery-action')),
      );
      await tester.pumpAndSettle();
      core.preflight = _preflight();
      await tester.tap(
        find.byKey(const ValueKey('preflight-manager-recovery-confirm')),
      );
      await tester.pumpAndSettle();

      expect(core.count('mgr_recover_install_v1'), 1);
      expect(core.count('mgr_undeploy_all'), 0);
    },
  );

  testWidgets(
    'deployment recovery confirmation cannot use a replaced preflight generation',
    (tester) async {
      final core = _PreflightCore(
        status: const {'state': 'recovery_required'},
        preflight: _preflight(
          findingId: 'install_mutation',
          findingAction: 'recover_deployment',
        ),
      );
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const ValueKey('preflight-status-action')));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const ValueKey('status-details-action-recover')),
      );
      await tester.pumpAndSettle();

      core.preflight = _preflight(
        findingId: 'install_mutation',
        findingCode: 'manager_mutation_recovery_required',
        findingAction: 'recover_manager_mutation',
        findingActionToken: 'guard-b-18',
      );
      final container = ProviderScope.containerOf(
        tester.element(find.byType(HomePage)),
      );
      container.read(preflightProvider.notifier).retry();
      await tester.pumpAndSettle();

      await tester.tap(find.widgetWithText(FilledButton, 'Recover'));
      await tester.pumpAndSettle();

      expect(core.count('mgr_undeploy_all'), 0);
      expect(core.count('mgr_recover_install_v1'), 0);
      expect(
        find.byKey(const ValueKey('preflight-manager-recovery-action')),
        findsOneWidget,
      );
    },
  );

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

  testWidgets(
    'install recovery opens bounded manual guidance and never mutates',
    (tester) async {
      final core = _PreflightCore(
        status: const {'state': 'recovery_required'},
        preflight: _preflight(
          findingId: 'install_mutation',
          findingAction: 'recover_install',
          findingDetail: 'A script build left recovery data behind.',
          findingItems: const [
            'recovery_journal: C:/games/a/.gore-as-compile-recovery',
            'shipping_cache_backup: C:/games/a/cache.gore-compile-bak',
          ],
        ),
      );
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();

      expect(
        find.byKey(const ValueKey('preflight-install-recovery-action')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('preflight-retry-action')),
        findsNothing,
      );
      expect(
        find.byKey(const ValueKey('preflight-status-action')),
        findsNothing,
      );

      await tester.tap(
        find.byKey(const ValueKey('preflight-install-recovery-action')),
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const ValueKey('preflight-install-recovery-dialog')),
        findsOneWidget,
      );
      expect(find.text('Installation recovery'), findsOneWidget);
      expect(find.textContaining('.gore-as-compile-recovery'), findsOneWidget);
      expect(core.count('mgr_apply'), 0);
      expect(core.count('mgr_undeploy_all'), 0);

      await tester.tap(
        find.byKey(const ValueKey('preflight-install-recovery-retry')),
      );
      await tester.pumpAndSettle();

      expect(core.count('mgr_preflight_v1'), 2);
      expect(core.count('mgr_apply'), 0);
      expect(core.count('mgr_undeploy_all'), 0);
    },
  );

  testWidgets(
    'bound interrupted Manager recovery confirms, reloads all views, and reports its outcome',
    (tester) async {
      final core = _PreflightCore(
        preflight: _preflight(
          findingId: 'install_mutation',
          findingCode: 'manager_mutation_recovery_required',
          findingAction: 'recover_manager_mutation',
          findingActionToken: 'guard-a-17',
          findingDetail: 'An interrupted Manager Apply can be recovered.',
        ),
      );
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();

      final initialLibraryReads = core.count('mgr_library_list');
      final initialStatusReads = core.count('mgr_status');
      final initialPreflightReads = core.count('mgr_preflight_v1');
      final initialConflictReads = core.count('mgr_analyze');
      final actionFinder = find.byKey(
        const ValueKey('preflight-manager-recovery-action'),
      );
      final action = tester.widget<TextButton>(actionFinder);
      action.focusNode!.requestFocus();
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const ValueKey('preflight-manager-recovery-dialog')),
        findsOneWidget,
      );
      expect(
        find.textContaining('Savegames are never changed'),
        findsOneWidget,
      );

      // The next preflight read must prove that recovery cleared the finding.
      core.preflight = _preflight();
      await tester.tap(
        find.byKey(const ValueKey('preflight-manager-recovery-confirm')),
      );
      await tester.pumpAndSettle();

      expect(core.count('mgr_recover_install_v1'), 1);
      final recoveryCall = core.calls.singleWhere(
        (call) => call.command == 'mgr_recover_install_v1',
      );
      expect(recoveryCall.payload, {
        'game_root': r'C:\games\a',
        'expected_guard_id': 'guard-a-17',
      });
      expect(core.count('mgr_library_list'), greaterThan(initialLibraryReads));
      expect(core.count('mgr_status'), greaterThan(initialStatusReads));
      expect(
        core.count('mgr_preflight_v1'),
        greaterThan(initialPreflightReads),
      );
      expect(core.count('mgr_analyze'), greaterThan(initialConflictReads));
      expect(
        find.textContaining('recorded baseline state was restored'),
        findsOneWidget,
      );
      expect(
        FocusManager.instance.primaryFocus?.debugLabel,
        'mod-manager-status-details',
      );
    },
  );

  testWidgets('manager recovery without its opaque token stays read-only', (
    tester,
  ) async {
    final core = _PreflightCore(
      preflight: _preflight(
        findingId: 'install_mutation',
        findingCode: 'manager_mutation_recovery_required',
        findingAction: 'recover_manager_mutation',
      ),
    );
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();

    expect(
      find.byKey(const ValueKey('preflight-manager-recovery-action')),
      findsNothing,
    );
    expect(
      find.byKey(const ValueKey('preflight-retry-action')),
      findsOneWidget,
    );
    expect(core.count('mgr_recover_install_v1'), 0);
  });

  testWidgets('busy recovery outcome is reported without claiming success', (
    tester,
  ) async {
    final core = _PreflightCore(
      preflight: _preflight(
        findingId: 'install_mutation',
        findingCode: 'manager_mutation_recovery_required',
        findingAction: 'recover_manager_mutation',
        findingActionToken: 'guard-a-17',
      ),
    )..recoveryOutcome = 'busy';
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const ValueKey('preflight-manager-recovery-action')),
    );
    await tester.pumpAndSettle();
    core.preflight = _preflight(
      findingId: 'install_mutation',
      findingAction: 'wait_for_install_mutation',
      findingDetail: 'The operation is active again.',
    );
    await tester.tap(
      find.byKey(const ValueKey('preflight-manager-recovery-confirm')),
    );
    await tester.pumpAndSettle();

    expect(find.textContaining('operation is active again'), findsWidgets);
    expect(
      find.textContaining('recorded baseline state was restored'),
      findsNothing,
    );
    expect(core.count('mgr_recover_install_v1'), 1);
  });

  testWidgets(
    'failed manager recovery survives reload as a localized bounded error',
    (tester) async {
      final core =
          _PreflightCore(
              preflight: _preflight(
                findingId: 'install_mutation',
                findingCode: 'manager_mutation_recovery_required',
                findingAction: 'recover_manager_mutation',
                findingActionToken: 'guard-a-17',
              ),
            )
            ..recoveryError =
                'recovery refused\n${List<String>.filled(700, 'x').join()}';
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();

      await tester.tap(
        find.byKey(const ValueKey('preflight-manager-recovery-action')),
      );
      await tester.pumpAndSettle();
      core.preflight = _preflight();
      await tester.tap(
        find.byKey(const ValueKey('preflight-manager-recovery-confirm')),
      );
      await tester.pumpAndSettle();

      final feedback = tester.widget<Text>(
        find.textContaining('Recovery could not be completed'),
      );
      expect(feedback.data, contains('recovery refused x'));
      expect(feedback.data, isNot(contains('\n')));
      expect(feedback.data, endsWith('…'));
      expect(core.count('mgr_library_list'), greaterThan(1));
      expect(core.count('mgr_status'), greaterThan(1));
      expect(core.count('mgr_preflight_v1'), greaterThan(1));
      expect(core.count('mgr_analyze'), greaterThan(1));
    },
  );

  testWidgets('manager recovery awaits the requested preflight generation', (
    tester,
  ) async {
    final interrupted = _preflight(
      findingId: 'install_mutation',
      findingCode: 'manager_mutation_recovery_required',
      findingAction: 'recover_manager_mutation',
      findingActionToken: 'guard-a-17',
    );
    final core = _PreflightCore(preflight: interrupted);
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();
    final initialPreflightReads = core.count('mgr_preflight_v1');

    final actionFinder = find.byKey(
      const ValueKey('preflight-manager-recovery-action'),
    );
    final action = tester.widget<TextButton>(actionFinder);
    action.focusNode!.requestFocus();
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();
    core.blockNextPreflight = true;
    await tester.tap(
      find.byKey(const ValueKey('preflight-manager-recovery-confirm')),
    );
    while (core.blockedPreflight == null) {
      await tester.pump();
    }
    await tester.pump();

    expect(
      find.textContaining('recorded baseline state was restored'),
      findsNothing,
    );
    expect(core.count('mgr_preflight_v1'), initialPreflightReads + 1);

    core.blockedPreflight!.complete(interrupted);
    await tester.pumpAndSettle();

    expect(core.count('mgr_preflight_v1'), initialPreflightReads + 2);
    expect(
      find.textContaining('recorded baseline state was restored'),
      findsOneWidget,
    );
    expect(action.focusNode!.hasFocus, isTrue);
  });

  testWidgets('recovery focus never moves from old token A to new token B', (
    tester,
  ) async {
    final core = _PreflightCore(
      preflight: _preflight(
        findingId: 'install_mutation',
        findingCode: 'manager_mutation_recovery_required',
        findingAction: 'recover_manager_mutation',
        findingActionToken: 'guard-a-17',
      ),
    )..recoveryOutcome = 'busy';
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();

    final actionFinder = find.byKey(
      const ValueKey('preflight-manager-recovery-action'),
    );
    final oldAction = tester.widget<TextButton>(actionFinder);
    oldAction.focusNode!.requestFocus();
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();

    core.preflight = _preflight(
      findingId: 'install_mutation',
      findingCode: 'manager_mutation_recovery_required',
      findingAction: 'recover_manager_mutation',
      findingActionToken: 'guard-b-18',
    );
    await tester.tap(
      find.byKey(const ValueKey('preflight-manager-recovery-confirm')),
    );
    await tester.pumpAndSettle();

    final newAction = tester.widget<TextButton>(actionFinder);
    expect(identical(newAction.focusNode, oldAction.focusNode), isTrue);
    expect(newAction.focusNode!.hasFocus, isFalse);
    expect(
      FocusManager.instance.primaryFocus?.debugLabel,
      'mod-manager-status-details',
    );
    expect(core.count('mgr_recover_install_v1'), 1);
  });

  testWidgets('confirmation cannot use a stale finding or replacement token', (
    tester,
  ) async {
    final core = _PreflightCore(
      preflight: _preflight(
        findingId: 'install_mutation',
        findingCode: 'manager_mutation_recovery_required',
        findingAction: 'recover_manager_mutation',
        findingActionToken: 'guard-a-17',
      ),
    );
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const ValueKey('preflight-manager-recovery-action')),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey('preflight-manager-recovery-dialog')),
      findsOneWidget,
    );

    core.preflight = _preflight(
      findingId: 'install_mutation',
      findingCode: 'manager_mutation_recovery_required',
      findingAction: 'recover_manager_mutation',
      findingActionToken: 'guard-b-18',
    );
    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    container.read(preflightProvider.notifier).retry();
    await container.read(preflightProvider.notifier).refresh();
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const ValueKey('preflight-manager-recovery-confirm')),
    );
    await tester.pumpAndSettle();

    expect(core.count('mgr_recover_install_v1'), 0);
    expect(
      find.byKey(const ValueKey('preflight-manager-recovery-action')),
      findsOneWidget,
    );
  });

  testWidgets(
    'active or retained install lock opens safe guidance and only rechecks',
    (tester) async {
      final lockState = _preflight(
        findingId: 'install_mutation',
        findingAction: 'wait_for_install_mutation',
        findingDetail: 'An install or script-build lock is present.',
        findingItems: const [
          'install_mutation_lock: C:/games/a/.gore-install-mutation.lock',
          'compile_lock: C:/games/a/.gore-as-compile.lock',
          'recovery_journal: C:/games/a/.gore-as-compile-recovery',
        ],
      );
      final core = _PreflightCore(preflight: lockState);
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();

      final actionFinder = find.byKey(
        const ValueKey('preflight-install-recovery-action'),
      );
      expect(actionFinder, findsOneWidget);
      expect(
        find.byKey(const ValueKey('preflight-retry-action')),
        findsNothing,
      );

      final action = tester.widget<TextButton>(actionFinder);
      action.focusNode!.requestFocus();
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const ValueKey('preflight-install-recovery-dialog')),
        findsOneWidget,
      );
      expect(find.textContaining('.gore-as-compile-recovery'), findsOneWidget);
      expect(find.textContaining('may still be running'), findsOneWidget);
      expect(core.count('mgr_apply'), 0);
      expect(core.count('mgr_undeploy_all'), 0);

      await tester.tap(
        find.byKey(const ValueKey('preflight-install-recovery-retry')),
      );
      await tester.pumpAndSettle();

      expect(core.count('mgr_preflight_v1'), 2);
      expect(
        tester.widget<TextButton>(actionFinder).focusNode!.hasFocus,
        isTrue,
      );
      expect(core.count('mgr_apply'), 0);
      expect(core.count('mgr_undeploy_all'), 0);
    },
  );

  testWidgets(
    'install-lock retry does not move focus across a changed action',
    (tester) async {
      final waiting = _preflight(
        findingId: 'install_mutation',
        findingAction: 'wait_for_install_mutation',
        findingDetail: 'An install operation may still be active.',
      );
      final core = _PreflightCore(preflight: waiting);
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();

      final actionFinder = find.byKey(
        const ValueKey('preflight-install-recovery-action'),
      );
      final action = tester.widget<TextButton>(actionFinder);
      action.focusNode!.requestFocus();
      await tester.pump();
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pumpAndSettle();

      core.blockNextPreflight = true;
      await tester.tap(
        find.byKey(const ValueKey('preflight-install-recovery-retry')),
      );
      for (var i = 0; i < 50 && core.blockedPreflight == null; i++) {
        await tester.pump(const Duration(milliseconds: 10));
      }
      expect(core.blockedPreflight, isNotNull);
      core.blockedPreflight!.complete(
        _preflight(
          findingId: 'install_mutation',
          findingAction: 'recover_install',
          findingDetail: 'Recovery is required.',
        ),
      );
      await tester.pumpAndSettle();

      final refreshed = tester.widget<TextButton>(actionFinder);
      expect(identical(refreshed.focusNode, action.focusNode), isFalse);
      expect(refreshed.focusNode!.hasFocus, isFalse);
      expect(core.count('mgr_preflight_v1'), 2);
      expect(core.count('mgr_apply'), 0);
      expect(core.count('mgr_undeploy_all'), 0);
    },
  );

  testWidgets(
    'stale recovery action cannot open old evidence after a new report publishes',
    (tester) async {
      final core = _PreflightCore(
        preflight: _preflight(
          findingId: 'install_mutation',
          findingAction: 'recover_install',
          findingDetail: 'A recovery is visible.',
          findingItems: const ['recovery_journal: C:/games/a/A-recovery'],
        ),
      );
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();

      final staleAction = find.byKey(
        const ValueKey('preflight-install-recovery-action'),
      );
      final container = ProviderScope.containerOf(
        tester.element(find.byType(HomePage)),
      );
      core.blockNextPreflight = true;
      container.read(preflightProvider.notifier).retry();
      for (var i = 0; i < 50 && core.blockedPreflight == null; i++) {
        await Future<void>.value();
      }
      expect(core.blockedPreflight, isNotNull);
      core.blockedPreflight!.complete(
        _preflight(
          findingId: 'install_mutation',
          findingAction: 'recover_install',
          findingDetail: 'B recovery is current.',
          findingItems: const ['recovery_journal: C:/games/a/B-recovery'],
        ),
      );
      for (var i = 0; i < 50; i++) {
        final current = container.read(preflightProvider);
        if (!current.busy &&
            current.report?.primarySetupFinding?.items.singleOrNull ==
                'recovery_journal: C:/games/a/B-recovery') {
          break;
        }
        await Future<void>.value();
      }
      final current = container.read(preflightProvider);
      expect(current.busy, isFalse);
      expect(current.report?.primarySetupFinding?.items, const [
        'recovery_journal: C:/games/a/B-recovery',
      ]);

      // No frame has been pumped since B published, so this still invokes the
      // callback captured by A's visible button.
      await tester.tap(staleAction);
      await tester.pumpAndSettle();

      expect(
        find.byKey(const ValueKey('preflight-install-recovery-dialog')),
        findsNothing,
      );
      expect(find.textContaining('A-recovery'), findsNothing);
      expect(find.textContaining('B recovery is current.'), findsOneWidget);
      expect(core.count('mgr_apply'), 0);
      expect(core.count('mgr_undeploy_all'), 0);
    },
  );

  testWidgets(
    'slow unchanged recovery retry restores keyboard focus to its new button',
    (tester) async {
      final recovery = _preflight(
        findingId: 'install_mutation',
        findingAction: 'recover_install',
        findingDetail: 'Recovery still needs attention.',
        findingItems: const [
          'recovery_journal: C:/games/a/.gore-as-compile-recovery',
        ],
      );
      final core = _PreflightCore(preflight: recovery);
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();

      final actionFinder = find.byKey(
        const ValueKey('preflight-install-recovery-action'),
      );
      final action = tester.widget<TextButton>(actionFinder);
      action.focusNode!.requestFocus();
      await tester.pump();
      expect(action.focusNode!.hasFocus, isTrue);

      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('preflight-install-recovery-dialog')),
        findsOneWidget,
      );

      core.blockNextPreflight = true;
      await tester.tap(
        find.byKey(const ValueKey('preflight-install-recovery-retry')),
      );
      for (var i = 0; i < 50 && core.blockedPreflight == null; i++) {
        await tester.pump(const Duration(milliseconds: 10));
      }
      expect(core.blockedPreflight, isNotNull);
      core.blockedPreflight!.complete(recovery);
      await tester.pumpAndSettle();

      final refreshed = tester.widget<TextButton>(actionFinder);
      expect(refreshed.focusNode!.hasFocus, isTrue);
      expect(core.count('mgr_preflight_v1'), 2);
      expect(core.count('mgr_apply'), 0);
      expect(core.count('mgr_undeploy_all'), 0);
    },
  );

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

  testWidgets('compact 200 percent keeps install recovery help reachable', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(700, 460);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final core = _PreflightCore(
      preflight: _preflight(
        findingId: 'install_mutation',
        findingAction: 'recover_install',
        findingDetail: 'Recovery data blocks installation changes.',
      ),
    );

    await tester.pumpWidget(
      _home(core, textScaler: const TextScaler.linear(2)),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);

    final action = find.byKey(
      const ValueKey('preflight-install-recovery-action'),
    );
    expect(action, findsOneWidget);
    expect(
      tester.getRect(action).overlaps(Offset.zero & tester.view.physicalSize),
      isTrue,
    );
    await tester.tap(action);
    await tester.pumpAndSettle();

    expect(
      find.byKey(const ValueKey('preflight-install-recovery-dialog')),
      findsOneWidget,
    );
    expect(find.text('Installation recovery'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'compact German 200 percent keeps manager recovery confirmation reachable',
    (tester) async {
      tester.view.physicalSize = const Size(1000, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final core = _PreflightCore(
        preflight: _preflight(
          findingId: 'install_mutation',
          findingCode: 'manager_mutation_recovery_required',
          findingAction: 'recover_manager_mutation',
          findingActionToken: 'guard-a-17',
        ),
      );

      await tester.pumpWidget(
        _home(
          core,
          textScaler: const TextScaler.linear(2),
          locale: const Locale('de'),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);

      await tester.tap(
        find.byKey(const ValueKey('preflight-manager-recovery-action')),
      );
      await tester.pumpAndSettle();

      tester.view.physicalSize = const Size(700, 460);
      await tester.pumpAndSettle();

      final confirm = find.byKey(
        const ValueKey('preflight-manager-recovery-confirm'),
      );
      expect(confirm, findsOneWidget);
      expect(
        tester
            .getRect(confirm)
            .overlaps(Offset.zero & tester.view.physicalSize),
        isTrue,
      );
      expect(tester.takeException(), isNull);
    },
  );
}
