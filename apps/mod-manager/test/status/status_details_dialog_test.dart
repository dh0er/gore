import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:gore_manager/library/domain/library_notifier.dart';
import 'package:gore_manager/library/domain/models.dart';
import 'package:gore_manager/status/domain/status_notifier.dart';
import 'package:gore_manager/status/ui/status_details_dialog.dart';

const _root = 'C:/game';

final _library = LibraryState(
  authoritative: true,
  mods: const [
    ModEntryMetaView(id: 'a', kind: 'goremod', name: 'Alpha'),
    ModEntryMetaView(id: 'b', kind: 'goremod', name: 'Beta'),
    ModEntryMetaView(id: 'c', kind: 'goremod', name: 'Gamma'),
  ],
  loadout: const LoadoutView(
    entries: [
      LoadoutEntryView(id: 'a'),
      LoadoutEntryView(id: 'b'),
    ],
  ),
);

StatusState _state(
  Map<String, Object?> status, {
  String? error,
  ApplyReportView? report,
}) => StatusState(
  status: ManagerStatusView.fromJson(status),
  statusRoot: _root,
  gameRoot: _root,
  error: error,
  lastReport: report,
  studioActive: status['state'] == 'studio_deploy_active',
);

Map<String, Object?> _ownedGroup(
  List<String> items, {
  int? total,
  bool? truncated,
}) => {
  'items': items,
  'total': total ?? items.length,
  'truncated': truncated ?? (total != null && total > items.length),
};

Map<String, Object?> _ownedEvidence({
  List<String> live = const [],
  List<String> backups = const [],
  List<String> additive = const [],
  List<String> ue4ss = const [],
  List<String> recovery = const [],
  int? recoveryTotal,
}) => {
  'live': _ownedGroup(live),
  'backups': _ownedGroup(backups),
  'additive': _ownedGroup(additive),
  'ue4ss': _ownedGroup(ue4ss),
  'recovery': _ownedGroup(recovery, total: recoveryTotal),
};

class _DialogHarness extends StatelessWidget {
  const _DialogHarness({
    required this.state,
    required this.currentRoot,
    required this.applyEnabled,
    required this.operationsBusy,
    required this.onResult,
    this.library,
    this.textScaler = TextScaler.noScaling,
  });

  final StatusState state;
  final String? currentRoot;
  final bool applyEnabled;
  final bool operationsBusy;
  final LibraryState? library;
  final ValueChanged<StatusDetailsResult?> onResult;
  final TextScaler textScaler;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
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
      home: Scaffold(
        body: Builder(
          builder: (context) => Center(
            child: FilledButton(
              key: const ValueKey('open-status-dialog'),
              onPressed: () async {
                onResult(
                  await showDialog<StatusDetailsResult>(
                    context: context,
                    builder: (_) => StatusDetailsDialog(
                      state: state,
                      currentRoot: currentRoot,
                      library: library ?? _library,
                      operationsBusy: operationsBusy,
                      applyEnabled: applyEnabled,
                    ),
                  ),
                );
              },
              child: const Text('Open'),
            ),
          ),
        ),
      ),
    );
  }
}

Future<StatusDetailsResult?> _open(
  WidgetTester tester,
  StatusState state, {
  String? currentRoot = _root,
  bool applyEnabled = true,
  bool operationsBusy = false,
  LibraryState? library,
  TextScaler textScaler = TextScaler.noScaling,
}) async {
  StatusDetailsResult? result;
  await tester.pumpWidget(
    _DialogHarness(
      state: state,
      currentRoot: currentRoot,
      applyEnabled: applyEnabled,
      operationsBusy: operationsBusy,
      library: library,
      textScaler: textScaler,
      onResult: (value) => result = value,
    ),
  );
  await tester.tap(find.byKey(const ValueKey('open-status-dialog')));
  await tester.pumpAndSettle();
  expect(find.byKey(const ValueKey('status-details-dialog')), findsOneWidget);
  return result;
}

void main() {
  testWidgets('nothing deployed explains the state and offers first Apply', (
    tester,
  ) async {
    StatusDetailsResult? result;
    await tester.pumpWidget(
      _DialogHarness(
        state: _state({'state': 'nothing_deployed'}),
        currentRoot: _root,
        applyEnabled: true,
        operationsBusy: false,
        onResult: (value) => result = value,
      ),
    );
    await tester.tap(find.byKey(const ValueKey('open-status-dialog')));
    await tester.pumpAndSettle();

    expect(
      find.text('No Manager deployment is installed for this game.'),
      findsOneWidget,
    );
    await tester.tap(find.byKey(const ValueKey('status-details-action-apply')));
    await tester.pumpAndSettle();
    expect(result?.action, StatusDetailsAction.apply);
    expect(result?.rootAtClick, _root);
  });

  testWidgets(
    'in sync shows authoritative order and last Apply names/warnings',
    (tester) async {
      await _open(
        tester,
        _state(
          {
            'state': 'in_sync',
            'loadout': [
              {'id': 'b', 'enabled': true},
              {'id': 'a', 'enabled': false},
            ],
          },
          report: const ApplyReportView(
            applied: ['Beta package', 'Alpha package'],
            warnings: ['Optional file was skipped'],
          ),
        ),
      );

      expect(
        find.byKey(const ValueKey('status-details-section-in-sync')),
        findsOneWidget,
      );
      expect(find.text('Beta'), findsOneWidget);
      expect(find.text('Alpha'), findsOneWidget);
      expect(find.text('Disabled'), findsOneWidget);
      expect(find.text('Beta package'), findsOneWidget);
      expect(find.text('Alpha package'), findsOneWidget);
      expect(find.text('Optional file was skipped'), findsOneWidget);
      expect(
        find.byKey(const ValueKey('status-details-action-apply')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'recorded ownership evidence is collapsed, complete, selectable, and sanitized',
    (tester) async {
      await _open(
        tester,
        _state({
          'state': 'in_sync',
          'loadout': <Object?>[],
          'manager_owned': _ownedEvidence(
            live: ['C:/game/G1R/safe\u202eevil\u0007.bin'],
            backups: ['C:/game/G1R/original.bin.gore-bak'],
            recovery: ['C:/game/gore-mod.deployed.json'],
            recoveryTotal: 3,
          ),
        }),
      );

      expect(find.text('Recorded ownership evidence'), findsOneWidget);
      expect(find.text('C:/game/G1R/safe evil .bin'), findsNothing);
      expect(find.textContaining('\u202e'), findsNothing);

      await tester.tap(
        find.byKey(const ValueKey('status-details-manager-owned')),
      );
      await tester.pumpAndSettle();

      for (final heading in [
        'Replaced game files',
        'Pristine backups',
        'Added pak and container files',
        'UE4SS mod directories',
        'Recovery files and holders',
      ]) {
        expect(find.text(heading), findsOneWidget);
      }
      expect(find.text('C:/game/G1R/safe evil .bin'), findsOneWidget);
      expect(find.text('C:/game/G1R/original.bin.gore-bak'), findsOneWidget);
      expect(find.text('C:/game/gore-mod.deployed.json'), findsOneWidget);
      expect(find.text('No paths recorded in this group.'), findsNWidgets(2));
      expect(find.text('1 of 3 recorded paths shown.'), findsOneWidget);
      expect(
        find.descendant(
          of: find.byKey(const ValueKey('status-details-owned-live-0')),
          matching: find.byType(SelectionArea),
        ),
        findsOneWidget,
      );
    },
  );

  testWidgets('unauthorized states ignore even syntactically valid evidence', (
    tester,
  ) async {
    for (final status in [
      {
        'state': 'nothing_deployed',
        'manager_owned': _ownedEvidence(live: ['C:/must-not-show']),
      },
      {
        'state': 'studio_deploy_active',
        'mod_name': 'Studio',
        'manager_owned': _ownedEvidence(live: ['C:/must-not-show']),
      },
      {
        'state': 'future_state',
        'manager_owned': _ownedEvidence(live: ['C:/must-not-show']),
      },
    ]) {
      await _open(tester, _state(status), applyEnabled: false);
      expect(
        find.byKey(const ValueKey('status-details-manager-owned')),
        findsNothing,
      );
      expect(find.text('C:/must-not-show'), findsNothing);
      await tester.tap(
        find.byKey(const ValueKey('status-details-action-close')),
      );
      await tester.pumpAndSettle();
    }
  });

  test('ownership copy is generated for all supported locales', () async {
    for (final locale in AppLocalizations.supportedLocales) {
      final l10n = await AppLocalizations.delegate.load(locale);
      expect(
        [
          l10n.statusDetailsOwnershipTitle,
          l10n.statusDetailsOwnershipDescription,
          l10n.statusDetailsOwnershipLive,
          l10n.statusDetailsOwnershipBackups,
          l10n.statusDetailsOwnershipAdditive,
          l10n.statusDetailsOwnershipUe4ss,
          l10n.statusDetailsOwnershipRecovery,
          l10n.statusDetailsOwnershipEmpty,
          l10n.statusDetailsOwnershipShown(1, 2),
        ].every((value) => value.trim().isNotEmpty),
        isTrue,
        reason: locale.toLanguageTag(),
      );
    }
  });

  testWidgets(
    'changes pending compares deployed and target and preserves disabled Apply semantics',
    (tester) async {
      await _open(
        tester,
        _state({
          'state': 'changes_pending',
          'deployed': [
            {'id': 'a', 'enabled': true},
          ],
          'target': [
            {'id': 'b', 'enabled': true},
            {'id': 'c', 'enabled': true},
          ],
        }),
        applyEnabled: false,
      );

      expect(find.text('Currently deployed'), findsOneWidget);
      expect(find.text('After Apply'), findsOneWidget);
      expect(find.text('Alpha'), findsOneWidget);
      expect(find.text('Beta'), findsOneWidget);
      expect(find.text('Gamma'), findsOneWidget);
      final apply = tester.widget<FilledButton>(
        find.byKey(const ValueKey('status-details-action-apply')),
      );
      expect(apply.onPressed, isNull);
    },
  );

  testWidgets('missing changes fields are unavailable rather than empty', (
    tester,
  ) async {
    await _open(
      tester,
      _state({'state': 'changes_pending'}),
      applyEnabled: false,
    );

    expect(
      find.text('The installed core did not provide these details.'),
      findsNWidgets(2),
    );
    expect(find.text('No mods in this loadout.'), findsNothing);
  });

  testWidgets('game updated lists drifted paths and returns Reapply', (
    tester,
  ) async {
    StatusDetailsResult? result;
    await tester.pumpWidget(
      _DialogHarness(
        state: _state({
          'state': 'game_updated',
          'drifted': ['G1R/Content/Paks/a.pak', 'G1R/bin/core.dll'],
        }),
        currentRoot: _root,
        applyEnabled: true,
        operationsBusy: false,
        onResult: (value) => result = value,
      ),
    );
    await tester.tap(find.byKey(const ValueKey('open-status-dialog')));
    await tester.pumpAndSettle();

    expect(find.text('G1R/Content/Paks/a.pak'), findsOneWidget);
    expect(find.text('G1R/bin/core.dll'), findsOneWidget);
    await tester.tap(
      find.byKey(const ValueKey('status-details-action-reapply')),
    );
    await tester.pumpAndSettle();
    expect(result?.action, StatusDetailsAction.apply);
    expect(result?.rootAtClick, _root);
  });

  testWidgets('studio status names the mod and returns Take over', (
    tester,
  ) async {
    StatusDetailsResult? result;
    await tester.pumpWidget(
      _DialogHarness(
        state: _state({
          'state': 'studio_deploy_active',
          'mod_name': 'Quest Preview',
        }),
        currentRoot: _root,
        applyEnabled: false,
        operationsBusy: false,
        onResult: (value) => result = value,
      ),
    );
    await tester.tap(find.byKey(const ValueKey('open-status-dialog')));
    await tester.pumpAndSettle();

    expect(find.text('Studio mod: Quest Preview'), findsOneWidget);
    await tester.tap(
      find.byKey(const ValueKey('status-details-action-take-over')),
    );
    await tester.pumpAndSettle();
    expect(result?.action, StatusDetailsAction.takeOver);
    expect(result?.rootAtClick, _root);
  });

  testWidgets('recovery required returns Recover', (tester) async {
    StatusDetailsResult? result;
    await tester.pumpWidget(
      _DialogHarness(
        state: _state({
          'state': 'recovery_required',
          'manager_owned': _ownedEvidence(
            recovery: ['C:/game/gore-mod.deployed.json'],
          ),
        }),
        currentRoot: _root,
        applyEnabled: false,
        operationsBusy: false,
        onResult: (value) => result = value,
      ),
    );
    await tester.tap(find.byKey(const ValueKey('open-status-dialog')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey('status-details-manager-owned')),
      findsOneWidget,
    );
    await tester.tap(
      find.byKey(const ValueKey('status-details-action-recover')),
    );
    await tester.pumpAndSettle();
    expect(result?.action, StatusDetailsAction.recover);
    expect(result?.rootAtClick, _root);
  });

  testWidgets('unknown and errored states retain technical detail and Refresh', (
    tester,
  ) async {
    await _open(
      tester,
      _state({'state': 'future_state'}, error: 'native detail'),
    );

    expect(
      find.text(
        'Deployment status could not be verified. Refresh before applying mods.',
      ),
      findsOneWidget,
    );
    expect(find.text('native detail'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('status-details-action-refresh')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('status-details-action-apply')),
      findsNothing,
    );
  });

  testWidgets('non-authoritative library offers Refresh for known status', (
    tester,
  ) async {
    await _open(
      tester,
      _state({'state': 'in_sync', 'loadout': <Object?>[]}),
      library: _library.copyWith(authoritative: false),
    );

    expect(
      find.byKey(const ValueKey('status-details-action-refresh')),
      findsOneWidget,
    );
  });

  testWidgets('library error offers Refresh even with authoritative data', (
    tester,
  ) async {
    await _open(
      tester,
      _state({'state': 'in_sync', 'loadout': <Object?>[]}),
      library: _library.copyWith(error: 'library detail'),
    );

    expect(
      find.byKey(const ValueKey('status-details-action-refresh')),
      findsOneWidget,
    );
  });

  testWidgets(
    'Studio fallback survives Unknown but a known non-Studio state clears it',
    (tester) async {
      await _open(
        tester,
        StatusState(
          status: ManagerStatusView.fromJson(const {'state': 'future_state'}),
          statusRoot: _root,
          gameRoot: _root,
          studioActive: true,
        ),
        applyEnabled: false,
      );

      expect(find.text('Deployment: Studio deployment active'), findsOneWidget);
      expect(
        find.byKey(const ValueKey('status-details-action-refresh')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('status-details-action-take-over')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const ValueKey('status-details-action-close')),
      );
      await tester.pumpAndSettle();

      await _open(
        tester,
        StatusState(
          status: ManagerStatusView.fromJson(const {
            'state': 'nothing_deployed',
          }),
          statusRoot: _root,
          gameRoot: _root,
          studioActive: true,
        ),
        applyEnabled: false,
      );

      expect(
        find.text('No Manager deployment is installed for this game.'),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('status-details-action-take-over')),
        findsNothing,
      );
    },
  );

  testWidgets('root mismatch hides stale errors and last Apply reports', (
    tester,
  ) async {
    await _open(
      tester,
      _state(
        {
          'state': 'in_sync',
          'loadout': <Object?>[],
          'manager_owned': _ownedEvidence(live: ['C:/old-root-owned.bin']),
        },
        error: 'old-root native detail',
        report: const ApplyReportView(
          applied: ['Old root mod'],
          warnings: ['Old root warning'],
        ),
      ),
      currentRoot: 'C:/other-game',
      applyEnabled: false,
    );

    expect(find.text('old-root native detail'), findsNothing);
    expect(find.text('Old root mod'), findsNothing);
    expect(find.text('Old root warning'), findsNothing);
    expect(find.text('Recorded ownership evidence'), findsNothing);
    expect(find.text('C:/old-root-owned.bin'), findsNothing);
    expect(
      find.byKey(const ValueKey('status-details-action-refresh')),
      findsOneWidget,
    );
  });

  testWidgets('null status authority is Unknown, not Nothing deployed', (
    tester,
  ) async {
    await _open(
      tester,
      const StatusState(gameRoot: _root),
      applyEnabled: false,
    );

    expect(
      find.text(
        'Deployment status could not be verified. Refresh before applying mods.',
      ),
      findsOneWidget,
    );
    expect(
      find.text('No Manager deployment is installed for this game.'),
      findsNothing,
    );
    expect(
      find.byKey(const ValueKey('status-details-action-refresh')),
      findsOneWidget,
    );
  });

  testWidgets('missing root offers Settings and never a mutation action', (
    tester,
  ) async {
    await _open(
      tester,
      const StatusState(error: StatusNotifier.noGamePath),
      currentRoot: null,
      applyEnabled: false,
    );

    expect(
      find.byKey(const ValueKey('status-details-action-settings')),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('status-details-action-refresh')),
      findsNothing,
    );
    expect(
      find.byKey(const ValueKey('status-details-action-apply')),
      findsNothing,
    );
  });

  testWidgets('100k loadout stays lazy with usable compact dialog actions', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(700, 460);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final target = [
      for (var i = 0; i < 100000; i++)
        {'id': i == 0 ? 'a' : 'mod-$i', 'enabled': i.isEven},
    ];

    await _open(
      tester,
      _state({
        'state': 'changes_pending',
        'deployed': <Object?>[],
        'target': target,
      }),
      textScaler: const TextScaler.linear(2),
    );
    expect(tester.takeException(), isNull);

    final listFinder = find.byKey(const ValueKey('status-details-list-target'));
    final list = tester.widget<ListView>(listFinder);
    final delegate = list.childrenDelegate as SliverChildBuilderDelegate;
    expect(delegate.estimatedChildCount, target.length);
    final builtRows = find.byWidgetPredicate((widget) {
      final key = widget.key;
      return key is ValueKey<String> &&
          key.value.startsWith('status-loadout-target-');
    });
    expect(builtRows.evaluate().length, lessThan(100));
    expect(find.text('Alpha'), findsOneWidget);
    expect(
      find.byKey(const ValueKey('status-details-action-close')).hitTestable(),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('status-details-action-apply')).hitTestable(),
      findsOneWidget,
    );
  });

  testWidgets('long status data scrolls at 700x460 and 200 percent text', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(700, 460);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final drifted = [
      for (var i = 0; i < 100000; i++) 'G1R/Content/file-$i.bin',
    ];

    await _open(
      tester,
      _state({'state': 'game_updated', 'drifted': drifted}),
      textScaler: const TextScaler.linear(2),
    );
    expect(tester.takeException(), isNull);
    final listFinder = find.byKey(
      const ValueKey('status-details-list-drifted'),
    );
    final list = tester.widget<ListView>(listFinder);
    final delegate = list.childrenDelegate as SliverChildBuilderDelegate;
    expect(delegate.estimatedChildCount, drifted.length);
    final builtRows = find.byWidgetPredicate((widget) {
      final key = widget.key;
      return key is ValueKey<String> &&
          key.value.startsWith('status-details-drifted-');
    });
    expect(builtRows.evaluate().length, lessThan(100));
    expect(
      find.byKey(const ValueKey('status-details-drifted-0')),
      findsOneWidget,
    );
    final scrollable = find.descendant(
      of: listFinder,
      matching: find.byType(Scrollable),
    );
    tester.state<ScrollableState>(scrollable).position.jumpTo(1000);
    await tester.pump();
    expect(tester.takeException(), isNull);
    expect(
      find.byKey(const ValueKey('status-details-drifted-0')),
      findsNothing,
    );
    expect(
      find.byKey(const ValueKey('status-details-action-close')).hitTestable(),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey('status-details-action-reapply')).hitTestable(),
      findsOneWidget,
    );
  });

  testWidgets('128 recorded paths stay lazy at compact 200 percent text', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(700, 460);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final paths = [
      for (var i = 0; i < 128; i++) 'C:/game/G1R/Content/owned-$i.bin',
    ];

    await _open(
      tester,
      _state({
        'state': 'in_sync',
        'loadout': <Object?>[],
        'manager_owned': _ownedEvidence(live: paths),
      }),
      textScaler: const TextScaler.linear(2),
    );
    final expansion = find.byKey(
      const ValueKey('status-details-manager-owned'),
    );
    await tester.ensureVisible(expansion);
    await tester.tap(expansion);
    await tester.pumpAndSettle();

    final listFinder = find.byKey(
      const ValueKey('status-details-list-owned-live'),
    );
    final list = tester.widget<ListView>(listFinder);
    final delegate = list.childrenDelegate as SliverChildBuilderDelegate;
    expect(delegate.estimatedChildCount, 128);
    final builtRows = find.byWidgetPredicate((widget) {
      final key = widget.key;
      return key is ValueKey<String> &&
          key.value.startsWith('status-details-owned-live-');
    });
    expect(builtRows.evaluate().length, lessThan(100));
    expect(
      find.byKey(const ValueKey('status-details-action-close')).hitTestable(),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });
}
