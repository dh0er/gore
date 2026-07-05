import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/conflicts/ui/conflict_panel.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/home_page.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:path/path.dart' as p;

/// Two mods that together produce one hard audio conflict when analyzed.
Map<String, Object?> _libraryList() => {
      'ok': true,
      'mods': [
        {
          'id': 'mod-a',
          'kind': 'goremod',
          'name': 'Better Torches',
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

/// A settings store that reports a fixed exe path so the game root resolves.
class _FixedSettingsStore implements UiSettingsStore {
  _FixedSettingsStore(this.exePath);
  final String exePath;
  @override
  UiSettings read() => UiSettings(gameExePath: exePath);
  @override
  void write(UiSettings settings) {}
}

/// Create a temp game tree whose exe path resolves via gameRootFromExe (it
/// looks for a sibling `G1R/` directory). Returns the exe path.
String _makeGameExe() {
  final root = Directory.systemTemp.createTempSync('gm_widget_test');
  addTearDown(() {
    if (root.existsSync()) root.deleteSync(recursive: true);
  });
  Directory(p.join(root.path, 'G1R', 'Binaries', 'Win64'))
      .createSync(recursive: true);
  return p.join(root.path, 'G1R', 'Binaries', 'Win64', 'G1R-Win64-Shipping.exe');
}

Widget _appWith(FakeGoreCoreFfiService fake, {String? exePath}) {
  return ProviderScope(
    overrides: [
      coreServiceProvider.overrideWithValue(fake),
      if (exePath != null)
        uiSettingsStoreProvider
            .overrideWithValue(_FixedSettingsStore(exePath)),
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
  testWidgets('mod list renders both mods and a hard conflict badge',
      (tester) async {
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
  });

  testWidgets('apply is disabled on nothing_deployed with no game path',
      (tester) async {
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

  testWidgets('conflict panel groups by target and bolds the winner',
      (tester) async {
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

  testWidgets('tapping the studio chip opens the take-over dialog',
      (tester) async {
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
    expect(find.widgetWithText(FilledButton, l10n.takeOverAction),
        findsOneWidget);
    // The title now appears twice (chip + dialog).
    expect(find.text(l10n.takeOverTitle), findsNWidgets(2));
    expect(tester.takeException(), isNull);
  });
}
