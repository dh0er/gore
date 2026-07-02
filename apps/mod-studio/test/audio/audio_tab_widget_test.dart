import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:gore_mod/app/domain/ui_settings.dart';
import 'package:gore_mod/audio/domain/audio_samples_provider.dart';
import 'package:gore_mod/audio/ui/audio_tab.dart';
import 'package:gore_mod/core/mod_ffi.dart';
import 'package:gore_mod/l10n/app_localizations.dart';

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

void main() {
  testWidgets('bank TabBar and categorized SFX split view', (tester) async {
    final samplesByBank = <String, List<AudioSampleInfo>>{
      'SFX.bank': [
        _sample('SFX_CREA_Wolf_Growl_01'),
        _sample('SFX_MAGIC_Impact_01'),
        _sample('taiko_hit'),
      ],
      'Music.bank': [_sample('MUS_Theme_01')],
      'CINEMATICS.bank': [],
      'VO.bank': [],
    };

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          uiSettingsStoreProvider.overrideWith(
            (ref) => _MemUiSettingsStore(
              const UiSettings(
                gameExePath:
                    r'C:\game\G1R\Binaries\Win64\G1R-Win64-Shipping.exe',
              ),
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
}
