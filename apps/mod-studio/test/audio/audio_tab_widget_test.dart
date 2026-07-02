import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/ui_settings.dart';
import 'package:gore_mod/audio/domain/audio_samples_provider.dart';
import 'package:gore_mod/audio/ui/audio_tab.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/l10n/app_localizations.dart';
import 'package:path/path.dart' as p;

/// Fake exe path built with host-native separators: gameRootFromExe resolves
/// the root via p.dirname walking up to the `G1R` path segment (no existsSync
/// hit is needed), and a `C:\...` literal never splits on POSIX runners.
final String _fakeGameExePath = p.join(
  Platform.isWindows ? r'C:\game' : '/game',
  'G1R',
  'Binaries',
  'Win64',
  'G1R-Win64-Shipping.exe',
);

/// In-memory settings store so the test never touches the real settings file.
class _MemUiSettingsStore implements UiSettingsStore {
  _MemUiSettingsStore(this._settings);
  UiSettings _settings;

  @override
  UiSettings read() => _settings;

  @override
  void write(UiSettings settings) => _settings = settings;
}

AudioSampleInfo _sample(String name) => AudioSampleInfo(
      index: 0,
      name: name,
      freq: 48000,
      channels: 2,
      seconds: 1.0,
    );

/// Pumps [AudioTab] with the settings store and sample provider faked, so the
/// test never touches the real settings file or FFI.
Future<void> _pumpAudioTab(
  WidgetTester tester,
  Map<String, List<AudioSampleInfo>> samplesByBank,
) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        uiSettingsStoreProvider.overrideWith(
          (ref) => _MemUiSettingsStore(
            UiSettings(gameExePath: _fakeGameExePath),
          ),
        ),
        audioSamplesProvider.overrideWith(
          (ref, bankFullPath) async =>
              samplesByBank[bankFullPath.split(RegExp(r'[\\/]')).last] ??
              const [],
        ),
      ],
      child: MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: const Scaffold(body: AudioTab()),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('bank TabBar and categorized SFX split view', (tester) async {
    await _pumpAudioTab(tester, {
      'SFX.bank': [
        _sample('SFX_CREA_Wolf_Growl_01'),
        _sample('SFX_MAGIC_Impact_01'),
        _sample('taiko_hit'),
      ],
      'Music.bank': [_sample('MUS_Theme_01')],
      'CINEMATICS.bank': [],
      'VO.bank': [],
    });

    // The bank TabBar is constrained to the 560px browser pane instead of
    // spanning the whole tab (test surface is 800px wide).
    expect(
      tester.getSize(find.byType(TabBar)).width,
      lessThanOrEqualTo(560),
    );

    // One tab per moddable bank, extension stripped, CINEMATICS title-cased.
    expect(find.widgetWithText(Tab, 'SFX'), findsOneWidget);
    expect(find.widgetWithText(Tab, 'Music'), findsOneWidget);
    expect(find.widgetWithText(Tab, 'Cinematics'), findsOneWidget);
    expect(find.widgetWithText(Tab, 'VO'), findsOneWidget);

    // SFX split view: sidebar shows categories with counts; only the selected
    // (first available) category's samples are listed.
    expect(find.text('Creatures (1)'), findsOneWidget);
    expect(find.text('Magic (1)'), findsOneWidget);
    expect(find.text('Other (1)'), findsOneWidget);
    expect(find.text('SFX_CREA_Wolf_Growl_01'), findsOneWidget);
    expect(find.text('SFX_MAGIC_Impact_01'), findsNothing);

    // Selecting another category swaps the sample list.
    await tester.tap(find.text('Magic (1)'));
    await tester.pumpAndSettle();
    expect(find.text('SFX_MAGIC_Impact_01'), findsOneWidget);
    expect(find.text('SFX_CREA_Wolf_Growl_01'), findsNothing);

    // Searching hides the sidebar and searches the whole bank.
    await tester.enterText(find.byType(TextField), 'taiko');
    await tester.pumpAndSettle();
    expect(find.text('Creatures (1)'), findsNothing);
    expect(find.text('taiko_hit'), findsOneWidget);

    // Switching banks clears the search: Music shows its flat whole-bank
    // list (no sidebar) instead of a stale 'taiko' filter.
    await tester.tap(find.widgetWithText(Tab, 'Music'));
    await tester.pumpAndSettle();
    expect(
      tester.widget<TextField>(find.byType(TextField)).controller!.text,
      isEmpty,
    );
    expect(find.text('MUS_Theme_01'), findsOneWidget);
    expect(find.text('Creatures (1)'), findsNothing);

    // Back on SFX the sidebar returns (search stayed cleared).
    await tester.tap(find.widgetWithText(Tab, 'SFX'));
    await tester.pumpAndSettle();
    expect(find.text('Creatures (1)'), findsOneWidget);
  });

  testWidgets('sample lists render alphabetically, case-insensitive',
      (tester) async {
    // Samples deliberately shuffled relative to alphabetical order: raw FSB
    // bank order would render them exactly as given here.
    await _pumpAudioTab(tester, {
      'SFX.bank': [
        _sample('SFX_CREA_Wolf_Growl_01'),
        _sample('SFX_CREA_Bloodfly_Idle_01'),
        _sample('SFX_CREA_Molerat_Attack_01'),
      ],
      'Music.bank': [
        _sample('MUS_Zen_01'),
        _sample('mus_battle_01'),
        _sample('MUS_Ambient_01'),
      ],
      'CINEMATICS.bank': [],
      'VO.bank': [],
    });

    // Orders [names] by their rendered vertical position (top to bottom).
    List<String> renderedOrder(List<String> names) => [...names]..sort(
          (a, b) => tester
              .getTopLeft(find.text(a))
              .dy
              .compareTo(tester.getTopLeft(find.text(b)).dy),
        );

    // SFX category list (Creatures bucket, selected by default).
    expect(
      renderedOrder([
        'SFX_CREA_Wolf_Growl_01',
        'SFX_CREA_Bloodfly_Idle_01',
        'SFX_CREA_Molerat_Attack_01',
      ]),
      [
        'SFX_CREA_Bloodfly_Idle_01',
        'SFX_CREA_Molerat_Attack_01',
        'SFX_CREA_Wolf_Growl_01',
      ],
    );

    // Non-SFX bank flat list, mixed-case names sorted case-insensitively.
    await tester.tap(find.widgetWithText(Tab, 'Music'));
    await tester.pumpAndSettle();
    expect(
      renderedOrder(['MUS_Zen_01', 'mus_battle_01', 'MUS_Ambient_01']),
      ['MUS_Ambient_01', 'mus_battle_01', 'MUS_Zen_01'],
    );
  });
}
