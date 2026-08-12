import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/shared_config.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/app/game_paths.dart';
import 'package:gore_manager/conflicts/ui/conflict_panel.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/home_page.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:gore_manager/status/domain/status_notifier.dart';
import 'package:path/path.dart' as p;

const _exeA = 'C:/games/a/G1R/Binaries/Win64/G1R-Win64-Shipping.exe';
const _exeB = 'C:/games/b/G1R/Binaries/Win64/G1R-Win64-Shipping.exe';

class _SettingsStore implements UiSettingsStore {
  UiSettings value = const UiSettings();

  @override
  UiSettings read() => value;

  @override
  void write(UiSettings settings) => value = settings;
}

SharedConfig _config(String? gamePath) {
  final directory = Directory.systemTemp.createTempSync('gm_actionable_status');
  addTearDown(() {
    if (directory.existsSync()) directory.deleteSync(recursive: true);
  });
  final config = SharedConfig(File(p.join(directory.path, 'config.json')));
  if (gamePath != null) config.setGamePath(gamePath);
  return config;
}

class _HomeCore implements GoreCoreFfiService {
  _HomeCore(
    this.status, {
    this.applyWarnings = const ['Optional payload skipped'],
  });

  Map<String, Object?> status;
  List<String> applyWarnings;
  Completer<Map<String, Object?>>? pendingStatus;
  final calls = <({String command, Map<String, Object?> payload})>[];

  static const mods = [
    {'id': 'a', 'kind': 'goremod', 'name': 'Alpha', 'components': <Object?>[]},
    {'id': 'b', 'kind': 'goremod', 'name': 'Beta', 'components': <Object?>[]},
  ];

  @override
  bool get isAvailable => true;

  @override
  String get description => 'actionable-status-test';

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
          'mods': mods,
          'loadout': {
            'format': 1,
            'entries': [
              {'id': 'a', 'enabled': true},
              {'id': 'b', 'enabled': true},
            ],
          },
        };
      case 'mgr_analyze':
        return {'ok': true, 'conflicts': <Object?>[]};
      case 'mgr_status':
        final pending = pendingStatus;
        if (pending != null) {
          pendingStatus = null;
          return pending.future;
        }
        return {'ok': true, 'status': status};
      case 'mgr_preflight_v1':
        return fakeHealthyManagerPreflightResponse();
      case 'mgr_apply':
        return {
          'ok': true,
          'report': {
            'applied': ['Alpha', 'Beta'],
            'warnings': applyWarnings,
          },
        };
      case 'mgr_undeploy_all':
        status = {'state': 'nothing_deployed'};
        return {'ok': true, 'removed': 1};
      default:
        return {'ok': true};
    }
  }
}

class _OnPopObserver extends NavigatorObserver {
  VoidCallback? onPop;

  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) {
    final callback = onPop;
    onPop = null;
    callback?.call();
    super.didPop(route, previousRoute);
  }
}

Widget _home(
  _HomeCore core, {
  String? gamePath = _exeA,
  TextScaler textScaler = TextScaler.noScaling,
  NavigatorObserver? navigatorObserver,
}) {
  return ProviderScope(
    overrides: [
      coreServiceProvider.overrideWithValue(core),
      uiSettingsStoreProvider.overrideWithValue(_SettingsStore()),
      sharedConfigProvider.overrideWithValue(_config(gamePath)),
    ],
    child: MaterialApp(
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      navigatorObservers: navigatorObserver == null
          ? const []
          : [navigatorObserver],
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(context).copyWith(textScaler: textScaler),
        child: child!,
      ),
      home: const HomePage(),
    ),
  );
}

void main() {
  testWidgets(
    'status trigger is one live region, keyboard opens it, and focus returns',
    (tester) async {
      final semantics = tester.ensureSemantics();
      try {
        final core = _HomeCore({
          'state': 'in_sync',
          'loadout': [
            {'id': 'a', 'enabled': true},
          ],
        });
        await tester.pumpWidget(_home(core));
        await tester.pumpAndSettle();

        final node = tester.getSemantics(
          find.byKey(const ValueKey('status-details-semantics')),
        );
        expect(node.flagsCollection.isButton, isTrue);
        expect(node.flagsCollection.isLiveRegion, isTrue);
        final liveRegions = tester
            .widgetList<Semantics>(
              find.descendant(
                of: find.byKey(const ValueKey('status-details-semantics')),
                matching: find.byType(Semantics),
                matchRoot: true,
              ),
            )
            .where((widget) => widget.properties.liveRegion == true);
        expect(liveRegions, hasLength(1));
        expect(
          liveRegions.single.key,
          const ValueKey('status-details-semantics'),
        );

        final trigger = tester.widget<TextButton>(
          find.byKey(const ValueKey('status-details-trigger')),
        );
        trigger.focusNode!.requestFocus();
        await tester.pump();
        expect(trigger.focusNode!.hasFocus, isTrue);
        await tester.sendKeyEvent(LogicalKeyboardKey.enter);
        await tester.pumpAndSettle();
        expect(
          find.byKey(const ValueKey('status-details-dialog')),
          findsOneWidget,
        );
        expect(find.text('Deployed load order'), findsOneWidget);

        await tester.tap(
          find.byKey(const ValueKey('status-details-action-close')),
        );
        await tester.pumpAndSettle();
        expect(trigger.focusNode!.hasFocus, isTrue);
      } finally {
        semantics.dispose();
      }
    },
  );

  testWidgets('compact 700x460 at 200 percent keeps primary actions usable', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(700, 460);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final core = _HomeCore({
      'state': 'game_updated',
      'drifted': ['G1R/Content/a.pak'],
    });
    await tester.pumpWidget(
      _home(core, textScaler: const TextScaler.linear(2)),
    );
    await tester.pumpAndSettle();

    expect(
      find.byKey(const ValueKey('compact-manager-action-bar')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
    for (final key in [
      'import-mod-action',
      'apply-loadout-action',
      'status-details-trigger',
    ]) {
      final rect = tester.getRect(find.byKey(ValueKey(key)));
      expect(rect.left, greaterThanOrEqualTo(0));
      expect(rect.right, lessThanOrEqualTo(700));
      expect(rect.top, greaterThanOrEqualTo(0));
      expect(rect.bottom, lessThanOrEqualTo(460));
      expect(rect.height, greaterThanOrEqualTo(40));
    }

    await tester.tap(find.byKey(const ValueKey('import-mod-action')));
    await tester.pumpAndSettle();
    expect(find.textContaining('Import folder'), findsOneWidget);
    await tester.tapAt(const Offset(10, 10));
    await tester.pumpAndSettle();
    expect(find.textContaining('Import folder'), findsNothing);

    await tester.tap(find.byKey(const ValueKey('status-details-trigger')));
    await tester.pumpAndSettle();
    for (final key in [
      'status-details-action-close',
      'status-details-action-reapply',
    ]) {
      final finder = find.byKey(ValueKey(key));
      expect(finder.hitTestable(), findsOneWidget);
      final rect = tester.getRect(finder);
      expect(rect.left, greaterThanOrEqualTo(0));
      expect(rect.right, lessThanOrEqualTo(700));
      expect(rect.top, greaterThanOrEqualTo(0));
      expect(rect.bottom, lessThanOrEqualTo(460));
    }
    expect(tester.takeException(), isNull);
    await tester.tap(find.byKey(const ValueKey('status-details-action-close')));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('apply-loadout-action')));
    await tester.pumpAndSettle();
    expect(
      core.calls.where((call) => call.command == 'mgr_apply'),
      hasLength(1),
    );

    await tester.tap(find.byType(ExpansionTile));
    await tester.pumpAndSettle();
    expect(tester.getSize(find.byType(ConflictPanel)).height, 72);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'open dialog drops stale root actions while replacement status is pending',
    (tester) async {
      final core = _HomeCore({
        'state': 'game_updated',
        'drifted': ['G1R/Content/old-root.pak'],
      });
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('status-details-trigger')));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('status-details-action-reapply')),
        findsOneWidget,
      );

      final pending = Completer<Map<String, Object?>>();
      core.pendingStatus = pending;
      final container = ProviderScope.containerOf(
        tester.element(find.byType(HomePage)),
      );
      container.read(gameExePathProvider.notifier).set(_exeB);
      await tester.pump();

      expect(find.text('G1R/Content/old-root.pak'), findsNothing);
      expect(
        find.byKey(const ValueKey('status-details-action-reapply')),
        findsNothing,
      );
      final refresh = tester.widget<TextButton>(
        find.byKey(const ValueKey('status-details-action-refresh')),
      );
      expect(refresh.onPressed, isNull);
      final toolbarApply = tester.widget<FilledButton>(
        find.byKey(const ValueKey('apply-loadout-action')),
      );
      expect(toolbarApply.onPressed, isNull);

      pending.complete({
        'ok': true,
        'status': {
          'state': 'changes_pending',
          'deployed': <Object?>[],
          'target': [
            {'id': 'b', 'enabled': true},
          ],
        },
      });
      await tester.pumpAndSettle();

      expect(find.text('After Apply'), findsOneWidget);
      final dialogApply = tester.widget<FilledButton>(
        find.byKey(const ValueKey('status-details-action-apply')),
      );
      expect(dialogApply.onPressed, isNotNull);
      expect(
        core.calls.where((call) => call.command == 'mgr_apply'),
        isEmpty,
        reason: 'a root switch never invokes a stale dialog action',
      );
    },
  );

  testWidgets('dialog action is rejected when root changes during pop', (
    tester,
  ) async {
    final observer = _OnPopObserver();
    final core = _HomeCore({
      'state': 'game_updated',
      'drifted': ['G1R/Content/a.pak'],
    });
    await tester.pumpWidget(_home(core, navigatorObserver: observer));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('status-details-trigger')));
    await tester.pumpAndSettle();

    final pending = Completer<Map<String, Object?>>();
    core.pendingStatus = pending;
    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    observer.onPop = () {
      container.read(gameExePathProvider.notifier).set(_exeB);
    };
    await tester.tap(
      find.byKey(const ValueKey('status-details-action-reapply')),
    );
    await tester.pump();

    expect(core.calls.where((call) => call.command == 'mgr_apply'), isEmpty);
    pending.complete({
      'ok': true,
      'status': {'state': 'nothing_deployed'},
    });
    await tester.pumpAndSettle();
  });

  testWidgets('recovery confirmation rechecks the selected root before write', (
    tester,
  ) async {
    final core = _HomeCore({'state': 'recovery_required'});
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('status-details-trigger')));
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const ValueKey('status-details-action-recover')),
    );
    await tester.pumpAndSettle();
    expect(
      find.text(
        'Recover the interrupted deployment and remove any partially deployed files?',
      ),
      findsOneWidget,
    );

    final pending = Completer<Map<String, Object?>>();
    core.pendingStatus = pending;
    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    container.read(gameExePathProvider.notifier).set(_exeB);
    await tester.pump();
    await tester.tap(find.widgetWithText(FilledButton, 'Recover'));
    await tester.pump();

    expect(
      core.calls.where((call) => call.command == 'mgr_undeploy_all'),
      isEmpty,
    );
    pending.complete({
      'ok': true,
      'status': {'state': 'nothing_deployed'},
    });
    await tester.pumpAndSettle();
  });

  testWidgets('undeploy confirmation rechecks busy state before write', (
    tester,
  ) async {
    final core = _HomeCore({'state': 'nothing_deployed'});
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('manager-overflow-action')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Undeploy all').last);
    await tester.pumpAndSettle();
    expect(
      find.text('Remove everything the manager deployed from the game?'),
      findsOneWidget,
    );

    final pending = Completer<Map<String, Object?>>();
    core.pendingStatus = pending;
    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    unawaited(
      container.read(statusProvider.notifier).refresh(gameRootFromExe(_exeA)),
    );
    await tester.pump();
    await tester.tap(find.widgetWithText(FilledButton, 'Undeploy all'));
    await tester.pump();

    expect(
      core.calls.where((call) => call.command == 'mgr_undeploy_all'),
      isEmpty,
    );
    pending.complete({
      'ok': true,
      'status': {'state': 'nothing_deployed'},
    });
    await tester.pumpAndSettle();
  });

  testWidgets('undeploy confirmation rechecks the selected root before write', (
    tester,
  ) async {
    final core = _HomeCore({'state': 'nothing_deployed'});
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const ValueKey('manager-overflow-action')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Undeploy all').last);
    await tester.pumpAndSettle();

    final pending = Completer<Map<String, Object?>>();
    core.pendingStatus = pending;
    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    container.read(gameExePathProvider.notifier).set(_exeB);
    await tester.pump();
    await tester.tap(find.widgetWithText(FilledButton, 'Undeploy all'));
    await tester.pump();

    expect(
      core.calls.where((call) => call.command == 'mgr_undeploy_all'),
      isEmpty,
    );
    pending.complete({
      'ok': true,
      'status': {'state': 'nothing_deployed'},
    });
    await tester.pumpAndSettle();
  });

  testWidgets('overflow undeploy rejects a root switch while popup closes', (
    tester,
  ) async {
    final observer = _OnPopObserver();
    final core = _HomeCore({'state': 'nothing_deployed'});
    await tester.pumpWidget(_home(core, navigatorObserver: observer));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('manager-overflow-action')));
    await tester.pumpAndSettle();

    final pending = Completer<Map<String, Object?>>();
    core.pendingStatus = pending;
    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    observer.onPop = () {
      container.read(gameExePathProvider.notifier).set(_exeB);
    };
    await tester.tap(find.text('Undeploy all').last);
    await tester.pump(const Duration(seconds: 1));

    expect(
      find.text('Remove everything the manager deployed from the game?'),
      findsNothing,
    );
    expect(
      core.calls.where((call) => call.command == 'mgr_undeploy_all'),
      isEmpty,
    );
    pending.complete({
      'ok': true,
      'status': {'state': 'nothing_deployed'},
    });
    await tester.pumpAndSettle();
  });

  testWidgets(
    'open overflow menu keeps Undeploy bound to its fully replaced root',
    (tester) async {
      final core = _HomeCore({'state': 'nothing_deployed'});
      await tester.pumpWidget(_home(core));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('manager-overflow-action')));
      await tester.pumpAndSettle();

      final container = ProviderScope.containerOf(
        tester.element(find.byType(HomePage)),
      );
      container.read(gameExePathProvider.notifier).set(_exeB);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('manager-operation-progress')),
        findsNothing,
      );
      expect(find.text('Undeploy all'), findsOneWidget);

      await tester.tap(find.text('Undeploy all'));
      await tester.pumpAndSettle();

      expect(
        find.text('Remove everything the manager deployed from the game?'),
        findsNothing,
      );
      expect(
        core.calls.where((call) => call.command == 'mgr_undeploy_all'),
        isEmpty,
      );
    },
  );

  testWidgets('last Apply banner caps warnings while details retain all', (
    tester,
  ) async {
    final warnings = [for (var i = 0; i < 80; i++) 'Warning $i'];
    final core = _HomeCore({
      'state': 'changes_pending',
      'deployed': <Object?>[],
      'target': [
        {'id': 'a', 'enabled': true},
      ],
    }, applyWarnings: warnings);
    await tester.pumpWidget(_home(core));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('apply-loadout-action')));
    await tester.pumpAndSettle();

    expect(find.text('Warning 0'), findsOneWidget);
    expect(find.text('Warning 2'), findsOneWidget);
    expect(find.text('Warning 3'), findsNothing);
    expect(find.text('+77 more'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('status-details-trigger')));
    await tester.pumpAndSettle();
    final warningsList = tester.widget<ListView>(
      find.byKey(const ValueKey('status-details-list-warnings')),
    );
    final delegate =
        warningsList.childrenDelegate as SliverChildBuilderDelegate;
    expect(delegate.estimatedChildCount, warnings.length);
  });

  testWidgets('no-root status details route to Settings without a mutation', (
    tester,
  ) async {
    final core = _HomeCore({'state': 'nothing_deployed'});
    await tester.pumpWidget(_home(core, gamePath: null));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('status-details-trigger')));
    await tester.pumpAndSettle();
    await tester.tap(
      find.byKey(const ValueKey('status-details-action-settings')),
    );
    await tester.pumpAndSettle();

    expect(find.text('Game executable'), findsOneWidget);
    final picker = tester.widget<OutlinedButton>(
      find.byKey(const ValueKey('settings-game-exe-pick')),
    );
    expect(picker.focusNode?.hasFocus, isTrue);
    expect(
      core.calls.where(
        (call) =>
            call.command == 'mgr_apply' || call.command == 'mgr_undeploy_all',
      ),
      isEmpty,
    );
  });

  testWidgets(
    'compact Settings route focuses and reveals a pre-scrolled game picker',
    (tester) async {
      tester.view.physicalSize = const Size(700, 600);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final core = _HomeCore({'state': 'nothing_deployed'});
      await tester.pumpWidget(
        _home(core, gamePath: null, textScaler: const TextScaler.linear(2)),
      );
      await tester.pumpAndSettle();

      await tester.tap(find.text('Settings'));
      await tester.pumpAndSettle();
      final settingsList = find.byKey(const ValueKey('settings-scroll-view'));
      final settingsScrollable = find.descendant(
        of: settingsList,
        matching: find.byWidgetPredicate(
          (widget) =>
              widget is Scrollable &&
              axisDirectionToAxis(widget.axisDirection) == Axis.vertical,
        ),
      );
      final position = tester
          .state<ScrollableState>(settingsScrollable)
          .position;
      position.jumpTo(position.maxScrollExtent);
      await tester.pump();
      final pickerFinder = find.byKey(const ValueKey('settings-game-exe-pick'));
      if (pickerFinder.hitTestable().evaluate().isNotEmpty) {
        position.jumpTo(0);
        await tester.pump();
      }
      expect(pickerFinder.hitTestable(), findsNothing);
      final preRouteScrollOffset = position.pixels;

      await tester.tap(find.text('Mods'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('status-details-trigger')));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const ValueKey('status-details-action-settings')),
      );
      await tester.pumpAndSettle();

      final picker = tester.widget<OutlinedButton>(pickerFinder);
      expect(picker.focusNode?.hasFocus, isTrue);
      expect(pickerFinder.hitTestable(), findsOneWidget);
      final revealedPosition = tester
          .state<ScrollableState>(
            find.descendant(
              of: settingsList,
              matching: find.byWidgetPredicate(
                (widget) =>
                    widget is Scrollable &&
                    axisDirectionToAxis(widget.axisDirection) == Axis.vertical,
              ),
            ),
          )
          .position;
      expect(revealedPosition.pixels, isNot(preRouteScrollOffset));
      final rect = tester.getRect(pickerFinder);
      expect(rect.left, greaterThanOrEqualTo(0));
      expect(rect.right, lessThanOrEqualTo(700));
      expect(rect.top, greaterThanOrEqualTo(0));
      expect(rect.bottom, lessThanOrEqualTo(600));
      expect(tester.takeException(), isNull);
    },
  );
}
