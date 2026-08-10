import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/shared_config.dart';
import 'package:gore_manager/app/domain/ui_settings.dart'
    show sharedConfigProvider;
import 'package:gore_manager/conflicts/ui/conflict_panel.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/home_page.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:gore_manager/library/domain/library_notifier.dart';
import 'package:gore_manager/library/ui/detail_panel.dart';
import 'package:gore_manager/library/ui/mod_list.dart';
import 'package:gore_manager/status/domain/status_notifier.dart';
import 'package:path/path.dart' as p;

/// Two mods that together produce one hard audio conflict when analyzed.
Map<String, Object?> _libraryList({String firstName = 'Better Torches'}) => {
  'ok': true,
  'mods': [
    {
      'id': 'mod-a',
      'kind': 'goremod',
      'name': firstName,
      'version': '1.2.0',
      'author': 'dh',
      'components': [
        {
          'type': 'audio_patch',
          'rel': 'audio/sfx.json',
          'targets': ['SFX|torch'],
        },
      ],
    },
    {
      'id': 'mod-b',
      'kind': 'foreign_pak',
      'name': 'Loud Pack',
      'components': [
        {
          'type': 'audio_patch',
          'rel': 'audio/sfx.json',
          'targets': ['SFX|torch'],
        },
      ],
    },
  ],
  'loadout': {
    'format': 1,
    'entries': [
      {'id': 'mod-a', 'enabled': true},
      {'id': 'mod-b', 'enabled': true},
    ],
  },
};

/// Mutable native-core stand-in for the remove flow. It returns a library
/// without mod-a after a successful mgr_remove, matching the real FFI contract.
class _RemoveCore implements GoreCoreFfiService {
  _RemoveCore({
    this.failRemove = false,
    this.partialFailure = false,
    this.blockRemove = false,
    this.blockInitialLibrary = false,
    this.failReloadAfterRemove = false,
    this.statusState = 'in_sync',
    this.firstName = 'Better Torches',
  });

  final bool failRemove;
  final bool partialFailure;
  final bool blockRemove;
  final bool blockInitialLibrary;
  bool failReloadAfterRemove;
  final String statusState;
  final String firstName;
  bool removed = false;
  bool failStatus = false;
  bool blockStatus = false;
  final initialLibraryStarted = Completer<void>();
  final releaseInitialLibrary = Completer<void>();
  final statusStarted = Completer<void>();
  final releaseStatus = Completer<void>();
  final removeStarted = Completer<void>();
  final releaseRemove = Completer<void>();
  final List<({String command, Map<String, Object?> payload})> calls = [];

  @override
  bool get isAvailable => true;

  @override
  String get description => 'remove-test';

  @override
  Future<Map<String, Object?>> execute(
    String command, {
    Map<String, Object?> payload = const {},
  }) async {
    calls.add((command: command, payload: payload));
    if (command == 'mgr_library_list' && !removed && blockInitialLibrary) {
      if (!initialLibraryStarted.isCompleted) initialLibraryStarted.complete();
      await releaseInitialLibrary.future;
    }
    if (command == 'mgr_remove') {
      if (blockRemove) {
        removeStarted.complete();
        await releaseRemove.future;
      }
      if (partialFailure) {
        removed = true;
        return {
          'ok': false,
          'error': {'code': 'IO', 'message': 'loadout update failed'},
        };
      }
      if (failRemove) {
        return {
          'ok': false,
          'error': {'code': 'IO', 'message': 'remove failed'},
        };
      }
      return _removeSuccess();
    }
    if (command == 'mgr_library_list' && removed && failReloadAfterRemove) {
      return {
        'ok': false,
        'error': {'code': 'IO', 'message': 'authoritative reload failed'},
      };
    }
    if (command == 'mgr_status' && failStatus) {
      return {
        'ok': false,
        'error': {'code': 'IO', 'message': 'status refresh failed'},
      };
    }
    if (command == 'mgr_status' && blockStatus) {
      if (!statusStarted.isCompleted) statusStarted.complete();
      await releaseStatus.future;
    }
    return switch (command) {
      'mgr_library_list' when !removed => _libraryList(firstName: firstName),
      'mgr_library_list' => {
        'ok': true,
        'mods': [
          {
            'id': 'mod-b',
            'kind': 'foreign_pak',
            'name': 'Loud Pack',
            'components': const [],
          },
        ],
        'loadout': {
          'format': 1,
          'entries': [
            if (partialFailure) {'id': 'mod-a', 'enabled': true},
            {'id': 'mod-b', 'enabled': true},
          ],
        },
      },
      'mgr_analyze' => {'ok': true, 'conflicts': const []},
      'mgr_status' => {
        'ok': true,
        'status': {'state': removed ? 'changes_pending' : statusState},
      },
      'mgr_set_loadout' => {'ok': true},
      _ => {
        'ok': false,
        'error': {'code': 'UNKNOWN', 'message': 'unknown command'},
      },
    };
  }

  Map<String, Object?> _removeSuccess() {
    removed = true;
    return {'ok': true, 'removed': true};
  }
}

Map<String, Object?> _oneHardConflict() => {
  'ok': true,
  'conflicts': [
    {
      'kind': 'audio',
      'target': 'SFX|torch',
      'mods': ['mod-a', 'mod-b'],
      'severity': 'hard',
    },
  ],
};

/// A shared config, backed by its own temp file, seeded with a fixed exe path
/// so the game root resolves. Isolated per call so tests don't share state.
SharedConfig _fixedSharedConfig(String exePath) {
  final dir = Directory.systemTemp.createTempSync('gm_widget_test_cfg');
  addTearDown(() {
    if (dir.existsSync()) dir.deleteSync(recursive: true);
  });
  final config = SharedConfig(File(p.join(dir.path, 'config.json')));
  config.setGamePath(exePath);
  return config;
}

/// Create a temp game tree whose exe path resolves via gameRootFromExe (it
/// looks for a sibling `G1R/` directory). Returns the exe path.
String _makeGameExe() {
  final root = Directory.systemTemp.createTempSync('gm_widget_test');
  addTearDown(() {
    if (root.existsSync()) root.deleteSync(recursive: true);
  });
  Directory(
    p.join(root.path, 'G1R', 'Binaries', 'Win64'),
  ).createSync(recursive: true);
  return p.join(
    root.path,
    'G1R',
    'Binaries',
    'Win64',
    'G1R-Win64-Shipping.exe',
  );
}

Widget _appWith(GoreCoreFfiService fake, {String? exePath}) {
  return ProviderScope(
    overrides: [
      coreServiceProvider.overrideWithValue(fake),
      if (exePath != null)
        sharedConfigProvider.overrideWithValue(_fixedSharedConfig(exePath)),
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

Widget _detailAppWith(GoreCoreFfiService fake, {double textScale = 1}) {
  return ProviderScope(
    overrides: [coreServiceProvider.overrideWithValue(fake)],
    child: MaterialApp(
      localizationsDelegates: const [
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(
          context,
        ).copyWith(textScaler: TextScaler.linear(textScale)),
        child: child!,
      ),
      home: const Scaffold(body: DetailPanel()),
    ),
  );
}

void _expectRemoveAnnouncement(WidgetTester tester, String message) {
  expect(find.byKey(const ValueKey('remove-mod-feedback')), findsOneWidget);
  final liveRegion = find.descendant(
    of: find.byType(SnackBar),
    matching: find.byWidgetPredicate(
      (widget) => widget is Semantics && widget.properties.liveRegion == true,
      description: 'a live region',
    ),
  );
  expect(liveRegion, findsOneWidget);
  expect(
    tester.getSemantics(liveRegion),
    matchesSemantics(
      label: message,
      isLiveRegion: true,
      hasDismissAction: true,
      hasScrollDownAction: true,
      hasScrollUpAction: true,
    ),
  );
}

Finder _customSemanticsActionsInModList() => find.descendant(
  of: find.byType(ModList),
  matching: find.byWidgetPredicate(
    (widget) =>
        widget is Semantics &&
        (widget.properties.customSemanticsActions?.isNotEmpty ?? false),
    description: 'custom reorder semantics actions',
  ),
);

void main() {
  testWidgets('initial library load does not show an unknown-state warning', (
    tester,
  ) async {
    final core = _RemoveCore(blockInitialLibrary: true);
    await tester.pumpWidget(_appWith(core));
    await core.initialLibraryStarted.future;
    await tester.pump();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    expect(find.text(l10n.libraryStateUnknown), findsNothing);
    expect(find.byKey(const ValueKey('library-refresh-action')), findsNothing);

    core.releaseInitialLibrary.complete();
    await tester.pumpAndSettle();
    expect(find.text('Better Torches'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('mod list renders both mods and a hard conflict badge', (
    tester,
  ) async {
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': _libraryList(),
        'mgr_analyze': _oneHardConflict(),
        'mgr_status': {
          'ok': true,
          'status': {'state': 'nothing_deployed'},
        },
      },
    );
    await tester.pumpWidget(_appWith(fake));
    await tester.pumpAndSettle();

    expect(find.text('Better Torches'), findsOneWidget);
    expect(find.text('Loud Pack'), findsOneWidget);
    // The hard conflict surfaces a red warning badge on each of the two mods.
    expect(find.byIcon(Icons.warning_amber_rounded), findsWidgets);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'remove confirms deployment impact, clears selection, and refreshes status',
    (tester) async {
      final semantics = tester.ensureSemantics();
      final exe = _makeGameExe();
      final core = _RemoveCore();
      await tester.pumpWidget(_appWith(core, exePath: exe));
      await tester.pumpAndSettle();

      final l10n = await AppLocalizations.delegate.load(const Locale('en'));
      await tester.tap(find.text('Better Torches').first);
      await tester.pumpAndSettle();

      final statusCallsBefore = core.calls
          .where((call) => call.command == 'mgr_status')
          .length;
      await tester.tap(find.byKey(const ValueKey('remove-mod-action')));
      await tester.pumpAndSettle();

      expect(
        find.text(l10n.removeModConfirm('Better Torches')),
        findsOneWidget,
      );
      expect(find.text(l10n.removeModDeploymentHint), findsOneWidget);

      await tester.tap(find.widgetWithText(FilledButton, l10n.removeModAction));
      await tester.pumpAndSettle();

      final removeCall = core.calls.singleWhere(
        (call) => call.command == 'mgr_remove',
      );
      expect(removeCall.payload, {'id': 'mod-a'});
      expect(find.text('Better Torches'), findsNothing);
      final container = ProviderScope.containerOf(
        tester.element(find.byType(HomePage)),
      );
      expect(container.read(selectedModProvider), isNull);
      _expectRemoveAnnouncement(
        tester,
        l10n.removeModSuccess('Better Torches'),
      );
      final importButton = tester.widget<OutlinedButton>(
        find.byKey(const ValueKey('import-mod-action')),
      );
      expect(importButton.focusNode?.hasFocus, isTrue);
      expect(
        core.calls.where((call) => call.command == 'mgr_status').length,
        statusCallsBefore + 1,
      );
      semantics.dispose();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'in-flight remove preserves a newly selected mod and does not steal focus',
    (tester) async {
      final core = _RemoveCore(blockRemove: true);
      await tester.pumpWidget(_appWith(core));
      await tester.pumpAndSettle();

      final l10n = await AppLocalizations.delegate.load(const Locale('en'));
      await tester.tap(find.text('Better Torches').first);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('remove-mod-action')));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, l10n.removeModAction));
      await core.removeStarted.future;
      await tester.pump();

      final container = ProviderScope.containerOf(
        tester.element(find.byType(HomePage)),
      );
      await tester.tap(find.text('Loud Pack').first);
      await tester.pump();
      expect(container.read(selectedModProvider), 'mod-b');

      core.releaseRemove.complete();
      await tester.pumpAndSettle();

      expect(container.read(selectedModProvider), 'mod-b');
      expect(find.text('Loud Pack'), findsWidgets);
      expect(find.text('Better Torches'), findsNothing);
      final importButton = tester.widget<OutlinedButton>(
        find.byKey(const ValueKey('import-mod-action')),
      );
      expect(importButton.focusNode?.hasFocus, isFalse);
      expect(
        find.byKey(const ValueKey('library-refresh-action')),
        findsNothing,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('failed remove keeps the selected mod available for retry', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final core = _RemoveCore(failRemove: true);
    await tester.pumpWidget(_appWith(core));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    await tester.tap(find.text('Better Torches').first);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('remove-mod-action')));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, l10n.removeModAction));
    await tester.pumpAndSettle();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    expect(container.read(selectedModProvider), 'mod-a');
    expect(find.text('Better Torches'), findsWidgets);
    expect(find.byKey(const ValueKey('remove-mod-action')), findsOneWidget);
    expect(find.textContaining('remove failed'), findsWidgets);
    _expectRemoveAnnouncement(
      tester,
      l10n.removeModFailed('Better Torches', 'mgr_remove: remove failed'),
    );
    expect(
      core.calls.where((call) => call.command == 'mgr_remove'),
      hasLength(1),
    );
    semantics.dispose();
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'partial remove reloads truth, warns honestly, and keeps Apply disabled',
    (tester) async {
      final semantics = tester.ensureSemantics();
      final exe = _makeGameExe();
      final core = _RemoveCore(partialFailure: true);
      await tester.pumpWidget(_appWith(core, exePath: exe));
      await tester.pumpAndSettle();

      final l10n = await AppLocalizations.delegate.load(const Locale('en'));
      await tester.tap(find.text('Better Torches').first);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('remove-mod-action')));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, l10n.removeModAction));
      await tester.pumpAndSettle();

      final container = ProviderScope.containerOf(
        tester.element(find.byType(HomePage)),
      );
      final library = container.read(libraryProvider);
      expect(library.authoritative, isTrue);
      expect(library.error, contains('loadout update failed'));
      expect(library.modById('mod-a'), isNull);
      expect(container.read(selectedModProvider), isNull);
      expect(find.text('Better Torches'), findsNothing);
      expect(
        tester
            .widget<FilledButton>(
              find.widgetWithText(FilledButton, l10n.actionApply),
            )
            .onPressed,
        isNull,
      );
      final refreshButton = tester.widget<TextButton>(
        find.byKey(const ValueKey('library-refresh-action')),
      );
      expect(refreshButton.onPressed, isNotNull);
      expect(refreshButton.focusNode?.hasFocus, isTrue);
      _expectRemoveAnnouncement(
        tester,
        l10n.removeModPartialFailure(
          'Better Torches',
          'mgr_remove: loadout update failed',
        ),
      );

      await tester.tap(find.byKey(const ValueKey('library-refresh-action')));
      await tester.pumpAndSettle();
      expect(container.read(libraryProvider).error, isNull);
      expect(
        find.byKey(const ValueKey('library-refresh-action')),
        findsNothing,
      );
      expect(
        tester
            .widget<FilledButton>(
              find.widgetWithText(FilledButton, l10n.actionApply),
            )
            .onPressed,
        isNotNull,
      );
      semantics.dispose();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('status error disables Apply and manual Refresh reads once', (
    tester,
  ) async {
    final exe = _makeGameExe();
    final core = _RemoveCore(statusState: 'changes_pending');
    await tester.pumpWidget(_appWith(core, exePath: exe));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    FilledButton applyButton() => tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, l10n.actionApply),
    );
    expect(applyButton().onPressed, isNotNull);
    final statusCallsBefore = core.calls
        .where((call) => call.command == 'mgr_status')
        .length;

    core.failStatus = true;
    await tester.tap(find.byIcon(Icons.more_vert));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.refreshAction));
    await tester.pumpAndSettle();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    expect(
      container.read(statusProvider).error,
      contains('status refresh failed'),
    );
    expect(container.read(statusProvider).status, isNull);
    expect(container.read(statusProvider).statusRoot, isNull);
    expect(find.text(l10n.statusUnknown), findsOneWidget);
    expect(find.text(l10n.statusNothingDeployed), findsNothing);
    expect(applyButton().onPressed, isNull);
    expect(
      core.calls.where((call) => call.command == 'mgr_status').length,
      statusCallsBefore + 1,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'unknown remove outcome keeps selection and marks conflicts unverified',
    (tester) async {
      final semantics = tester.ensureSemantics();
      final exe = _makeGameExe();
      final core = _RemoveCore(
        partialFailure: true,
        failReloadAfterRemove: true,
      );
      await tester.pumpWidget(_appWith(core, exePath: exe));
      await tester.pumpAndSettle();

      final l10n = await AppLocalizations.delegate.load(const Locale('en'));
      await tester.tap(find.text('Better Torches').first);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('remove-mod-action')));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, l10n.removeModAction));
      await tester.pumpAndSettle();

      final container = ProviderScope.containerOf(
        tester.element(find.byType(HomePage)),
      );
      expect(container.read(libraryProvider).authoritative, isFalse);
      expect(container.read(selectedModProvider), 'mod-a');
      expect(find.text(l10n.conflictsUnverified), findsOneWidget);
      expect(find.text(l10n.noConflicts), findsNothing);
      _expectRemoveAnnouncement(
        tester,
        l10n.removeModOutcomeUnknown(
          'Better Torches',
          'mgr_remove: loadout update failed',
        ),
      );
      final refreshButton = tester.widget<TextButton>(
        find.byKey(const ValueKey('library-refresh-action')),
      );
      expect(refreshButton.onPressed, isNotNull);
      expect(refreshButton.focusNode?.hasFocus, isTrue);

      core.blockStatus = true;
      final libraryCallsBefore = core.calls
          .where((call) => call.command == 'mgr_library_list')
          .length;
      final statusRefresh = container
          .read(statusProvider.notifier)
          .refresh('game-root');
      await core.statusStarted.future;
      await tester.pump();
      expect(
        tester
            .widget<TextButton>(
              find.byKey(const ValueKey('library-refresh-action')),
            )
            .onPressed,
        isNull,
      );
      expect(
        find.byWidgetPredicate(
          (widget) =>
              widget.key == const ValueKey('manager-overflow-action') &&
              widget is PopupMenuButton &&
              !widget.enabled,
          description: 'disabled overflow menu',
        ),
        findsOneWidget,
      );
      expect(
        core.calls.where((call) => call.command == 'mgr_library_list').length,
        libraryCallsBefore,
      );
      core.releaseStatus.complete();
      await statusRefresh;
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<TextButton>(
              find.byKey(const ValueKey('library-refresh-action')),
            )
            .onPressed,
        isNotNull,
      );

      await tester.tap(find.byKey(const ValueKey('library-refresh-action')));
      await tester.pumpAndSettle();
      expect(container.read(libraryProvider).authoritative, isFalse);
      expect(
        tester
            .widget<TextButton>(
              find.byKey(const ValueKey('library-refresh-action')),
            )
            .focusNode
            ?.hasFocus,
        isTrue,
      );
      await tester.pump(const Duration(seconds: 5));
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.byType(ExpansionTile));
      await tester.tap(find.byType(ExpansionTile));
      await tester.pumpAndSettle();
      expect(find.text(l10n.conflictsUnverified), findsNWidgets(2));
      expect(find.text(l10n.noConflicts), findsNothing);
      expect(
        tester
            .widget<FilledButton>(
              find.widgetWithText(FilledButton, l10n.actionApply),
            )
            .onPressed,
        isNull,
      );

      core.failReloadAfterRemove = false;
      await tester.tap(find.byKey(const ValueKey('library-refresh-action')));
      await tester.pumpAndSettle();
      expect(container.read(libraryProvider).authoritative, isTrue);
      expect(container.read(selectedModProvider), isNull);

      semantics.dispose();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('blocked library mutation disables every mutation control', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final core = _RemoveCore(blockRemove: true);
    await tester.pumpWidget(_appWith(core));
    await tester.pumpAndSettle();

    expect(find.byType(ReorderableListView), findsOneWidget);
    expect(_customSemanticsActionsInModList(), findsWidgets);

    await tester.tap(find.text('Better Torches').first);
    await tester.pumpAndSettle();
    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    core.calls.clear();

    final removing = container.read(libraryProvider.notifier).remove('mod-a');
    await core.removeStarted.future;
    await tester.pump();

    expect(find.byType(ReorderableListView), findsNothing);
    expect(_customSemanticsActionsInModList(), findsNothing);

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
          .widgetList<Checkbox>(find.byType(Checkbox))
          .every((checkbox) => checkbox.onChanged == null),
      isTrue,
    );
    expect(
      tester
          .widgetList<ReorderableDragStartListener>(
            find.byType(ReorderableDragStartListener),
          )
          .every((handle) => !handle.enabled),
      isTrue,
    );
    expect(
      tester
          .widget<OutlinedButton>(
            find.byKey(const ValueKey('remove-mod-action')),
          )
          .onPressed,
      isNull,
    );

    await tester.tap(find.byType(Checkbox).first);
    await tester.drag(
      find.byIcon(Icons.drag_handle).first,
      const Offset(0, 80),
    );
    await tester.pump();
    expect(
      core.calls.where((call) => call.command == 'mgr_remove'),
      hasLength(1),
    );
    expect(
      core.calls.where((call) => call.command == 'mgr_set_loadout'),
      isEmpty,
    );

    core.releaseRemove.complete();
    await removing;
    await tester.pumpAndSettle();
    semantics.dispose();
    expect(tester.takeException(), isNull);
  });

  testWidgets('remove confirmation remains reachable at compact 200% scale', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(700, 460);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final core = _RemoveCore(
      firstName:
          'A very long community mod name that must wrap without hiding actions',
    );
    await tester.pumpWidget(_detailAppWith(core, textScale: 2));
    await tester.pumpAndSettle();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(DetailPanel)),
    );
    container.read(selectedModProvider.notifier).state = 'mod-a';
    await tester.pumpAndSettle();
    expect(container.read(selectedModProvider), 'mod-a');
    expect(container.read(libraryProvider).modById('mod-a'), isNotNull);
    await tester.scrollUntilVisible(
      find.byKey(const ValueKey('remove-mod-action')),
      160,
      scrollable: find.descendant(
        of: find.byType(DetailPanel),
        matching: find.byType(Scrollable),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('remove-mod-action')));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    expect(find.text(l10n.removeModDeploymentHint), findsOneWidget);
    expect(
      find.widgetWithText(TextButton, l10n.commonCancel).hitTestable(),
      findsOneWidget,
    );
    expect(
      find.widgetWithText(FilledButton, l10n.removeModAction).hitTestable(),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('apply is disabled when in sync', (tester) async {
    final exe = _makeGameExe();
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': _libraryList(),
        'mgr_analyze': {'ok': true, 'conflicts': []},
        'mgr_status': {
          'ok': true,
          'status': {'state': 'in_sync', 'loadout': []},
        },
      },
    );
    await tester.pumpWidget(_appWith(fake, exePath: exe));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    final applyBtn = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, l10n.actionApply),
    );
    expect(applyBtn.onPressed, isNull); // disabled
  });

  testWidgets(
    'apply is enabled on nothing_deployed with an enabled mod + game path',
    (tester) async {
      // Regression: the first-ever deploy. mgr_status reports nothing_deployed
      // (nothing deployed yet), the loadout has enabled mods, and a game path is
      // set — Apply must be enabled so the user can perform the first deploy.
      final exe = _makeGameExe();
      final fake = FakeGoreCoreFfiService(
        responses: {
          'mgr_library_list': _libraryList(), // both mods enabled
          'mgr_analyze': {'ok': true, 'conflicts': []},
          'mgr_status': {
            'ok': true,
            'status': {'state': 'nothing_deployed'},
          },
        },
      );
      await tester.pumpWidget(_appWith(fake, exePath: exe));
      await tester.pumpAndSettle();

      final l10n = await AppLocalizations.delegate.load(const Locale('en'));
      final applyBtn = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, l10n.actionApply),
      );
      expect(applyBtn.onPressed, isNotNull); // enabled
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('apply is disabled on nothing_deployed with no game path', (
    tester,
  ) async {
    // Without a game path there's nowhere to deploy, so Apply stays disabled
    // even though the loadout has enabled mods.
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': _libraryList(), // both mods enabled
        'mgr_analyze': {'ok': true, 'conflicts': []},
        'mgr_status': {
          'ok': true,
          'status': {'state': 'nothing_deployed'},
        },
      },
    );
    await tester.pumpWidget(_appWith(fake)); // no exePath
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    final applyBtn = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, l10n.actionApply),
    );
    expect(applyBtn.onPressed, isNull); // disabled
  });

  testWidgets('apply is enabled when changes are pending', (tester) async {
    final exe = _makeGameExe();
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': _libraryList(),
        'mgr_analyze': {'ok': true, 'conflicts': []},
        'mgr_status': {
          'ok': true,
          'status': {'state': 'changes_pending'},
        },
      },
    );
    await tester.pumpWidget(_appWith(fake, exePath: exe));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    final applyBtn = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, l10n.actionApply),
    );
    expect(applyBtn.onPressed, isNotNull); // enabled
  });

  testWidgets('future deployment state is shown as Unknown and cannot Apply', (
    tester,
  ) async {
    final exe = _makeGameExe();
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': _libraryList(),
        'mgr_analyze': {'ok': true, 'conflicts': []},
        'mgr_status': {
          'ok': true,
          'status': {'state': 'future_manager_state'},
        },
      },
    );
    await tester.pumpWidget(_appWith(fake, exePath: exe));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    expect(find.text(l10n.statusUnknown), findsOneWidget);
    expect(find.text(l10n.statusNothingDeployed), findsNothing);
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, l10n.actionApply),
          )
          .onPressed,
      isNull,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('known non-studio postflight does not offer stale takeover', (
    tester,
  ) async {
    final exe = _makeGameExe();
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': _libraryList(),
        'mgr_analyze': {'ok': true, 'conflicts': []},
        'mgr_status': {
          'ok': true,
          'status': {'state': 'nothing_deployed'},
        },
        'mgr_apply': {
          'ok': false,
          'error': {
            'code': 'STUDIO_DEPLOY_ACTIVE',
            'message': 'studio owned the install during apply',
          },
        },
      },
    );
    await tester.pumpWidget(_appWith(fake, exePath: exe));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    await tester.tap(find.widgetWithText(FilledButton, l10n.actionApply));
    await tester.pumpAndSettle();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(HomePage)),
    );
    expect(container.read(statusProvider).studioActive, isFalse);
    expect(find.text(l10n.statusNothingDeployed), findsOneWidget);
    expect(find.text(l10n.statusStudioDeploy), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('conflict panel groups by target and bolds the winner', (
    tester,
  ) async {
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': _libraryList(),
        'mgr_analyze': _oneHardConflict(),
        'mgr_status': {
          'ok': true,
          'status': {'state': 'nothing_deployed'},
        },
      },
    );
    await tester.pumpWidget(_appWith(fake));
    await tester.pumpAndSettle();

    // Expand the conflicts ExpansionTile.
    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    await tester.tap(find.text(l10n.conflictsTitle(1)).first);
    await tester.pumpAndSettle();

    // The group header shows the target, in the panel.
    expect(find.text('SFX|torch'), findsWidgets);
    // The conflict panel itself is present.
    expect(find.byType(ConflictPanel), findsOneWidget);

    // Winner is mod-b (last in loadout order = highest priority): its chip is
    // rendered with a bold RichText run tagged with the winner label.
    final winnerFinder = find.byWidgetPredicate((w) {
      if (w is! RichText) return false;
      final text = w.text.toPlainText();
      return text.contains('Loud Pack') && text.contains(l10n.conflictWinner);
    });
    expect(winnerFinder, findsWidgets);
    expect(tester.takeException(), isNull);
  });

  testWidgets('tapping the studio chip opens the take-over dialog', (
    tester,
  ) async {
    final exe = _makeGameExe();
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': _libraryList(),
        'mgr_analyze': {'ok': true, 'conflicts': []},
        'mgr_status': {
          'ok': true,
          'status': {'state': 'studio_deploy_active', 'mod_name': 'MyMod'},
        },
        'mgr_undeploy_all': {'ok': true, 'removed': 1},
      },
    );
    await tester.pumpWidget(_appWith(fake, exePath: exe));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    // Before tapping, the take-over dialog isn't shown.
    expect(find.text(l10n.takeOverBody), findsNothing);

    // The status chip shows the studio-deploy label; tap it. (The same label
    // is also the dialog title, so target the chip via its InkWell ancestor.)
    await tester.tap(
      find.ancestor(
        of: find.text(l10n.statusStudioDeploy),
        matching: find.byType(InkWell),
      ),
    );
    await tester.pumpAndSettle();

    // The dialog body + action are unique to the take-over dialog.
    expect(find.text(l10n.takeOverBody), findsOneWidget);
    expect(
      find.widgetWithText(FilledButton, l10n.takeOverAction),
      findsOneWidget,
    );
    // The title now appears twice (chip + dialog).
    expect(find.text(l10n.takeOverTitle), findsNWidgets(2));
    expect(tester.takeException(), isNull);
  });

  testWidgets('recovery chip opens the undeploy recovery path', (tester) async {
    final exe = _makeGameExe();
    final fake = FakeGoreCoreFfiService(
      responses: {
        'mgr_library_list': _libraryList(),
        'mgr_analyze': {'ok': true, 'conflicts': []},
        'mgr_status': {
          'ok': true,
          'status': {'state': 'recovery_required'},
        },
        'mgr_undeploy_all': {'ok': true, 'removed': 1},
      },
    );
    await tester.pumpWidget(_appWith(fake, exePath: exe));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    expect(find.text(l10n.statusRecoveryRequired), findsOneWidget);
    expect(find.text(l10n.statusNothingDeployed), findsNothing);
    final apply = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, l10n.actionApply),
    );
    expect(apply.onPressed, isNull);

    await tester.tap(
      find.ancestor(
        of: find.text(l10n.statusRecoveryRequired),
        matching: find.byType(InkWell),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text(l10n.recoveryRequiredConfirm), findsOneWidget);

    await tester.tap(find.widgetWithText(FilledButton, l10n.recoveryAction));
    await tester.pumpAndSettle();
    expect(
      fake.calls.any((call) => call.command == 'mgr_undeploy_all'),
      isTrue,
    );
    expect(tester.takeException(), isNull);
  });
}
