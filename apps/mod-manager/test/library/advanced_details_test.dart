import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_manager/app/domain/ui_settings.dart';
import 'package:gore_manager/conflicts/ui/conflict_panel.dart';
import 'package:gore_manager/core/core_service.dart';
import 'package:gore_manager/core/providers.dart';
import 'package:gore_manager/l10n/app_localizations.dart';
import 'package:gore_manager/library/domain/models.dart';
import 'package:gore_manager/library/ui/detail_panel.dart';
import 'package:gore_manager/library/ui/mod_labels.dart';
import 'package:gore_manager/library/ui/mod_list.dart';
import 'package:gore_manager/settings/ui/settings_tab.dart';
import 'package:gore_manager/settings/ui/update_settings.dart';

/// One mod whose single component both claims a footprint target and carries a
/// non-exact coverage grade, so the plain/advanced split is observable.
Map<String, Object?> _libraryList() => {
  'ok': true,
  'mods': [
    {
      'id': 'Loud.Pack',
      'kind': 'foreign_pak',
      'name': 'Loud Pack',
      'source': 'LoudPack.zip',
      'components': [
        {
          'type': 'audio_patch',
          'rel': 'audio/loud',
          'targets': ['SFX|Thunder'],
          'coverage': 'partial',
        },
      ],
    },
  ],
  'loadout': {
    'format': 1,
    'entries': [
      {'id': 'Loud.Pack', 'enabled': true},
    ],
  },
};

/// A mod that replaces each of the three game-wide files a `raw_file`
/// component can target.
Map<String, Object?> _rawFileLibrary() => {
  'ok': true,
  'mods': [
    {
      'id': 'Total.Overhaul',
      'kind': 'foreign_rawfile',
      'name': 'Total Overhaul',
      'components': [
        {'type': 'raw_file', 'rel': 'raw/loc.lcache', 'target_file': 'lcache'},
        {
          'type': 'raw_file',
          'rel': 'raw/scripts.Cache',
          'target_file': 'script_cache',
        },
        {
          'type': 'raw_file',
          'rel': 'raw/SFX.bank',
          'target_file': {
            'bank': {'name': 'SFX'},
          },
        },
      ],
    },
  ],
  'loadout': {
    'format': 1,
    'entries': [
      {'id': 'Total.Overhaul', 'enabled': true},
    ],
  },
};

/// A component whose path is far wider than the 380 px detail pane.
Map<String, Object?> _longPathLibrary() => {
  'ok': true,
  'mods': [
    {
      'id': 'Reposition',
      'kind': 'foreign_triplet',
      'name': 'GothicUIReposition',
      'components': [
        {
          'type': 'triplet',
          'rel_base': 'G1R/Content/Paks/GothicUIReposition_LongEnoughToClip_P',
          'targets': ['/Game/UI/Player/Player_Widget'],
          'coverage': 'advisory',
        },
      ],
    },
  ],
  'loadout': {
    'format': 1,
    'entries': [
      {'id': 'Reposition', 'enabled': true},
    ],
  },
};

Map<String, Object?> _emptyLibrary() => {
  'ok': true,
  'mods': <Object?>[],
  'loadout': {'format': 1, 'entries': <Object?>[]},
};

/// A settings store that records writes so persistence can be asserted.
class _RecordingSettingsStore implements UiSettingsStore {
  UiSettings _current = const UiSettings();
  final List<bool> writtenAdvanced = [];

  @override
  UiSettings read() => _current;

  @override
  void write(UiSettings settings) {
    _current = settings;
    writtenAdvanced.add(settings.advancedDetails);
  }
}

Widget _app(
  Widget child, {
  Map<String, Object?>? library,
  UiSettingsStore? store,
}) => ProviderScope(
  overrides: [
    coreServiceProvider.overrideWithValue(
      FakeGoreCoreFfiService(
        responses: {
          'mgr_library_list': library ?? _libraryList(),
          'mgr_analyze': {'ok': true, 'conflicts': <Object?>[]},
        },
      ),
    ),
    if (store != null) uiSettingsStoreProvider.overrideWithValue(store),
  ],
  child: MaterialApp(
    localizationsDelegates: const [
      AppLocalizations.delegate,
      GlobalMaterialLocalizations.delegate,
      GlobalWidgetsLocalizations.delegate,
      GlobalCupertinoLocalizations.delegate,
    ],
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  ),
);

void main() {
  testWidgets('the plain detail view hides the technical layer', (
    tester,
  ) async {
    await tester.pumpWidget(_app(const DetailPanel()));
    await tester.pumpAndSettle();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(DetailPanel)),
    );
    container.read(selectedModProvider.notifier).state = 'Loud.Pack';
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    // What the mod changes stays visible, named by its file…
    expect(find.text('${l10n.componentAudio} · loud'), findsOneWidget);
    // …while the footprint target, the coverage grade and the import source do
    // not, because nothing about them is actionable for a player.
    expect(find.text('SFX|Thunder'), findsNothing);
    expect(find.text(l10n.footprintTargetsPartial), findsNothing);
    expect(find.text(l10n.modDetailSource), findsNothing);

    container.read(advancedDetailsProvider.notifier).set(true);
    await tester.pumpAndSettle();

    expect(find.text('SFX|Thunder'), findsOneWidget);
    // Advanced swaps the file name for the full path that locates it on disk.
    expect(
      find.text('${l10n.componentKindAudioPatch} · audio/loud'),
      findsOneWidget,
    );
    expect(find.text(l10n.footprintTargetsPartial), findsOneWidget);
    expect(find.text(l10n.modDetailSource), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('a clean conflict panel says nothing extra in the plain view', (
    tester,
  ) async {
    await tester.pumpWidget(
      _app(const SizedBox(height: 240, child: ConflictPanel())),
    );
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    expect(find.text(l10n.noConflicts), findsOneWidget);
    expect(find.text(l10n.loadOrderDirection), findsNothing);
    expect(find.text(l10n.footprintCoverageScope), findsNothing);

    ProviderScope.containerOf(
      tester.element(find.byType(ConflictPanel)),
    ).read(advancedDetailsProvider.notifier).set(true);
    await tester.pumpAndSettle();

    expect(find.text(l10n.loadOrderDirection), findsOneWidget);
    expect(find.text(l10n.footprintCoverageScope), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('the Settings switch flips and persists advanced details', (
    tester,
  ) async {
    final store = _RecordingSettingsStore();
    await tester.pumpWidget(
      _app(SettingsTab(gamePathFocusNode: FocusNode()), store: store),
    );
    await tester.pumpAndSettle();

    // The card sits below the fold in a lazy ListView, so bring it into view
    // before reading it.
    final toggle = find.byKey(const ValueKey('settings-advanced-details'));
    await tester.scrollUntilVisible(
      toggle,
      120,
      scrollable: find
          .descendant(
            of: find.byKey(const ValueKey('settings-scroll-view')),
            matching: find.byType(Scrollable),
          )
          .first,
    );
    expect(tester.widget<SwitchListTile>(toggle).value, isFalse);

    await tester.tap(toggle);
    await tester.pumpAndSettle();

    expect(tester.widget<SwitchListTile>(toggle).value, isTrue);
    expect(store.writtenAdvanced, [true]);
    expect(store.read().advancedDetails, isTrue);
    expect(tester.takeException(), isNull);
  });

  testWidgets('Settings offers update controls', (tester) async {
    await tester.pumpWidget(
      _app(
        SettingsTab(gamePathFocusNode: FocusNode()),
        store: _RecordingSettingsStore(),
      ),
    );
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    expect(find.byType(UpdateSettingsCard), findsOneWidget);
    expect(find.text(l10n.updatesTitle), findsOneWidget);
    expect(find.text(l10n.checkForUpdatesAutomatically), findsOneWidget);
    expect(find.text(l10n.checkForUpdatesNow), findsOneWidget);

    // A test run is not a Windows release build, so the controls must be
    // disabled rather than offering an action that quietly does nothing.
    final toggle = find.byKey(const ValueKey('settings-auto-update-check'));
    expect(tester.widget<SwitchListTile>(toggle).onChanged, isNull);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const ValueKey('settings-check-updates-now')),
          )
          .onPressed,
      isNull,
    );
    expect(tester.takeException(), isNull);
  });

  test('the auto-update preference defaults on and round-trips', () {
    expect(const UiSettings().autoUpdateCheck, isTrue);
    // An older settings file has no such key; it must not read as "off".
    expect(UiSettings.fromJson(const {}).autoUpdateCheck, isTrue);
    expect(
      UiSettings.fromJson(const {'autoUpdateCheck': false}).autoUpdateCheck,
      isFalse,
    );
    expect(
      UiSettings.fromJson(
        const UiSettings(autoUpdateCheck: false).toJson(),
      ).autoUpdateCheck,
      isFalse,
    );
  });

  testWidgets('a raw-file component is named by what it replaces', (
    tester,
  ) async {
    await tester.pumpWidget(
      _app(const DetailPanel(), library: _rawFileLibrary()),
    );
    await tester.pumpAndSettle();

    ProviderScope.containerOf(
      tester.element(find.byType(DetailPanel)),
    ).read(selectedModProvider.notifier).state = 'Total.Overhaul';
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    // Each of these swaps one game-wide file wholesale. "File" said nothing.
    expect(find.text(l10n.rawTargetGameText), findsOneWidget);
    expect(find.text(l10n.rawTargetGameScripts), findsOneWidget);
    expect(find.text(l10n.rawTargetSoundBankNamed('SFX')), findsOneWidget);
    expect(find.text(l10n.componentKindRawFile), findsNothing);
    expect(tester.takeException(), isNull);
  });

  test('every known raw-file destination has a label', () async {
    final l10n = await AppLocalizations.delegate.load(const Locale('de'));
    for (final kind in const ['lcache', 'script_cache', 'bank']) {
      expect(
        rawFileTargetLabel(l10n, RawFileTargetView(kind: kind)),
        isNotNull,
        reason: '\$kind must be named, not left as "file"',
      );
    }
    // An unknown future destination falls back rather than inventing a name.
    expect(
      rawFileTargetLabel(l10n, const RawFileTargetView(kind: 'future')),
      isNull,
    );
  });

  test('the plain vocabulary has one word per part of the game', () async {
    for (final locale in AppLocalizations.supportedLocales) {
      final l10n = await AppLocalizations.delegate.load(locale);
      // Every container mechanism collapses to the same player-facing word, so
      // a mod can never show "files" next to "game files" next to
      // "game file package" and expect anyone to tell them apart.
      final containerKinds = [
        'loose_pak',
        'triplet',
        'file_patch',
        'pak_file_patch',
      ];
      final labels = {
        for (final kind in containerKinds)
          componentPlainLabel(
            l10n,
            ComponentView(kind: kind, coverage: FootprintCoverage.exact),
          ),
      };
      expect(
        labels,
        hasLength(1),
        reason: '${locale.toLanguageTag()}: container kinds must share a word',
      );
      expect(labels.single, l10n.componentGameFiles);

      // And a chip must never disagree with the row it belongs to.
      for (final kind in [
        ...containerKinds,
        'loc_patch',
        'audio_patch',
        'angel_script_patch',
        'texture_patch',
        'voice_archive_patch',
        'ue4ss_lua',
      ]) {
        final component = ComponentView(
          kind: kind,
          coverage: FootprintCoverage.exact,
        );
        expect(
          componentChips(l10n, [component]).single.label,
          componentPlainLabel(l10n, component),
          reason: '${locale.toLanguageTag()}: $kind chip vs row wording',
        );
      }
    }
  });

  testWidgets('a long path is readable in full, not clipped', (tester) async {
    await tester.pumpWidget(
      _app(const DetailPanel(), library: _longPathLibrary()),
    );
    await tester.pumpAndSettle();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(DetailPanel)),
    );
    container.read(advancedDetailsProvider.notifier).set(true);
    container.read(selectedModProvider.notifier).state = 'Reposition';
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    final heading = find.text(
      '${l10n.componentKindTriplet} · '
      'G1R/Content/Paks/GothicUIReposition_LongEnoughToClip_P',
    );
    expect(heading, findsOneWidget);
    // Truncating it would leave the path unreadable and uncopyable, so it must
    // wrap onto more lines instead.
    expect(tester.widget<Text>(heading).overflow, isNot(TextOverflow.ellipsis));
    expect(tester.getSize(heading).height, greaterThan(20));
    expect(
      find.ancestor(of: heading, matching: find.byType(SelectionArea)),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('an empty library explains what to do next', (tester) async {
    await tester.pumpWidget(_app(const ModList(), library: _emptyLibrary()));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    expect(find.text(l10n.libraryEmptyTitle), findsOneWidget);
    expect(find.text(l10n.libraryEmptyBody), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('an unselected detail pane invites a selection', (tester) async {
    await tester.pumpWidget(_app(const DetailPanel()));
    await tester.pumpAndSettle();

    final l10n = await AppLocalizations.delegate.load(const Locale('en'));
    expect(find.text(l10n.detailEmptyHint), findsOneWidget);
    // The old placeholder reused the tab caption, which said nothing.
    expect(find.text(l10n.tabMods), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('the empty library fits a short pane at 200 percent text', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(700, 400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      MediaQuery(
        data: const MediaQueryData(textScaler: TextScaler.linear(2)),
        child: _app(const ModList(), library: _emptyLibrary()),
      ),
    );
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
  });
}
